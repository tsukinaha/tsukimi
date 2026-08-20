use std::{
    cell::Cell,
    fs::File,
    os::{
        fd::OwnedFd,
        unix::fs::FileExt,
    },
    rc::Rc,
    sync::Arc,
};

use wl_proxy::{
    object::{
        Object,
        ObjectCoreApi,
    },
    protocols::wayland::{
        wl_buffer::{
            WlBuffer,
            WlBufferHandler,
        },
        wl_shm::{
            WlShm,
            WlShmError,
            WlShmFormat,
            WlShmHandler,
        },
        wl_shm_pool::{
            WlShmPool,
            WlShmPoolError,
            WlShmPoolHandler,
        },
    },
};

use super::{
    SharedState
};
use std::cell::RefCell;

struct ShmPoolState {
    file: File,
    size: Cell<u64>,
}

#[derive(Debug, Clone, Copy)]
pub enum ShmMemoryFormat { Argb8888, Xrgb8888 }
pub struct ShmFrame {
    pub width: i32,
    pub height: i32,
    pub stride: usize,
    pub format: ShmMemoryFormat,
    pub data: Vec<u8>,
}


pub struct BufferInfo {
    pub buffer: Rc<WlBuffer>,
    pool: Rc<ShmPoolState>,
    offset: u64,
    width: i32,
    height: i32,
    stride: usize,
    format: ShmMemoryFormat,
}

#[derive(Clone)]
pub(crate) struct ShmSnapshot {
    width: i32,
    height: i32,
    stride: usize,
    format: ShmMemoryFormat,
    data: Arc<[u8]>,
}

impl ShmSnapshot {
    pub(crate) fn to_frame(&self) -> ShmFrame {
        ShmFrame {
            width: self.width,
            height: self.height,
            stride: self.stride,
            format: self.format,
            data: self.data.to_vec(),
        }
    }
}

impl BufferInfo {
    pub fn snapshot(&self) -> Option<ShmSnapshot> {
        self.copy_snapshot()
            .map_err(|error| {
                tracing::error!(
                    buffer_id = self.buffer.unique_id(),
                    "wl-proxy-mpv: failed to copy wl_shm frame: {error}"
                );
            })
            .ok()
    }

    fn copy_snapshot(&self) -> std::io::Result<ShmSnapshot> {
        let width = usize::try_from(self.width).map_err(|_| invalid_shm_buffer())?;
        let height = usize::try_from(self.height).map_err(|_| invalid_shm_buffer())?;
        let row_bytes = width.checked_mul(4).ok_or_else(invalid_shm_buffer)?;
        let data_len = row_bytes
            .checked_mul(height)
            .ok_or_else(invalid_shm_buffer)?;
        let mut data = vec![0; data_len];

        for row in 0..height {
            let source_offset = self
                .offset
                .checked_add(
                    u64::try_from(row)
                        .ok()
                        .and_then(|row| row.checked_mul(self.stride as u64))
                        .ok_or_else(invalid_shm_buffer)?,
                )
                .ok_or_else(invalid_shm_buffer)?;
            let start = row * row_bytes;
            self.pool
                .file
                .read_exact_at(&mut data[start..start + row_bytes], source_offset)?;
        }

        Ok(ShmSnapshot {
            width: self.width,
            height: self.height,
            stride: row_bytes,
            format: self.format,
            data: data.into(),
        })
    }
}

fn shm_buffer_end(offset: u64, width: usize, height: usize, stride: usize) -> Option<u64> {
    if width == 0 || height == 0 {
        return None;
    }
    let row_bytes = width.checked_mul(4)?;
    if stride < row_bytes {
        return None;
    }
    let byte_len = height
        .checked_sub(1)?
        .checked_mul(stride)?
        .checked_add(row_bytes)?;
    offset.checked_add(u64::try_from(byte_len).ok()?)
}

fn invalid_shm_buffer() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid wl_shm buffer")
}

pub struct ShmHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl WlShmHandler for ShmHandler {
    fn handle_create_pool(
        &mut self, slf: &Rc<WlShm>, id: &Rc<WlShmPool>, fd: &Rc<OwnedFd>, size: i32,
    ) {
        let Some(size) = u64::try_from(size).ok().filter(|size| *size > 0) else {
            tracing::error!(size, "wl-proxy-mpv: invalid wl_shm pool size");
            post_shm_error(slf, WlShmError::INVALID_STRIDE, "invalid wl_shm pool size");
            return;
        };

        let owned_fd = match fd.try_clone() {
            Ok(fd) => fd,
            Err(error) => {
                tracing::error!("wl-proxy-mpv: failed to clone wl_shm pool fd: {error}");
                post_shm_error(slf, WlShmError::INVALID_FD, "invalid wl_shm pool fd");
                return;
            }
        };
        let file = File::from(owned_fd);
        if file.metadata().is_ok_and(|metadata| metadata.len() < size) {
            tracing::error!(size, "wl-proxy-mpv: wl_shm backing file is too small");
            post_shm_error(
                slf,
                WlShmError::INVALID_FD,
                "wl_shm backing file is too small",
            );
            return;
        }

        id.set_forward_to_server(false);
        id.set_handler(ShmPoolHandler {
            state: Rc::clone(&self.state),
            pool: Rc::new(ShmPoolState {
                file,
                size: Cell::new(size),
            }),
        });
    }
}

struct ShmPoolHandler {
    state: Rc<RefCell<SharedState>>,
    pool: Rc<ShmPoolState>,
}

impl WlShmPoolHandler for ShmPoolHandler {
    fn handle_create_buffer(
        &mut self, slf: &Rc<WlShmPool>, id: &Rc<WlBuffer>, offset: i32, width: i32, height: i32,
        stride: i32, format: WlShmFormat,
    ) {
        let Some(format) = (format == WlShmFormat::ARGB8888)
            .then_some(ShmMemoryFormat::Argb8888)
            .or_else(|| (format == WlShmFormat::XRGB8888).then_some(ShmMemoryFormat::Xrgb8888))
        else {
            tracing::error!(format = format.0, "wl-proxy-mpv: unsupported wl_shm format");
            post_pool_error(
                slf,
                WlShmPoolError::INVALID_FORMAT,
                "unsupported wl_shm format",
            );
            return;
        };

        let Some(offset) = u64::try_from(offset).ok() else {
            tracing::error!(offset, "wl-proxy-mpv: invalid wl_shm buffer offset");
            post_pool_error(
                slf,
                WlShmPoolError::INVALID_STRIDE,
                "invalid wl_shm buffer offset",
            );
            return;
        };
        let (Ok(width_usize), Ok(height_usize), Ok(stride)) = (
            usize::try_from(width),
            usize::try_from(height),
            usize::try_from(stride),
        ) else {
            tracing::error!(
                width,
                height,
                stride,
                "wl-proxy-mpv: invalid wl_shm dimensions"
            );
            post_pool_error(
                slf,
                WlShmPoolError::INVALID_STRIDE,
                "invalid wl_shm dimensions",
            );
            return;
        };
        let Some(end) = shm_buffer_end(offset, width_usize, height_usize, stride) else {
            tracing::error!(
                width,
                height,
                stride,
                "wl-proxy-mpv: invalid wl_shm buffer range"
            );
            post_pool_error(
                slf,
                WlShmPoolError::INVALID_STRIDE,
                "invalid wl_shm buffer range",
            );
            return;
        };
        if end > self.pool.size.get() {
            tracing::error!(
                end,
                pool_size = self.pool.size.get(),
                "wl-proxy-mpv: wl_shm buffer exceeds its pool"
            );
            post_pool_error(
                slf,
                WlShmPoolError::INVALID_STRIDE,
                "wl_shm buffer exceeds its pool",
            );
            return;
        }

        id.set_forward_to_server(false);
        id.set_handler(ShmBufferHandler {
            state: Rc::clone(&self.state),
        });
        let buffer_id = id.unique_id();
        let mut state = self.state.borrow_mut();
        state.shm_buffer_info.insert(
            buffer_id,
            BufferInfo {
                buffer: Rc::clone(id),
                pool: Rc::clone(&self.pool),
                offset,
                width,
                height,
                stride,
                format,
            },
        );
    }

    fn handle_destroy(&mut self, slf: &Rc<WlShmPool>) {
        slf.delete_id();
    }

    fn handle_resize(&mut self, slf: &Rc<WlShmPool>, size: i32) {
        let Some(size) = u64::try_from(size)
            .ok()
            .filter(|size| *size > self.pool.size.get())
        else {
            tracing::error!(size, "wl-proxy-mpv: invalid wl_shm pool resize");
            post_pool_error(
                slf,
                WlShmPoolError::INVALID_STRIDE,
                "invalid wl_shm pool resize",
            );
            return;
        };
        if self
            .pool
            .file
            .metadata()
            .is_ok_and(|metadata| metadata.len() >= size)
        {
            self.pool.size.set(size);
        } else {
            tracing::error!(
                size,
                "wl-proxy-mpv: resized wl_shm backing file is too small"
            );
            post_pool_error(
                slf,
                WlShmPoolError::INVALID_STRIDE,
                "resized wl_shm backing file is too small",
            );
        }
    }
}

struct ShmBufferHandler {
    state: Rc<RefCell<SharedState>>,
}

impl WlBufferHandler for ShmBufferHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlBuffer>) {
        self.state
            .borrow_mut()
            .shm_buffer_info
            .remove(&slf.unique_id());
        slf.delete_id();
    }
}

fn post_shm_error(slf: &Rc<WlShm>, error: WlShmError, message: &str) {
    if let Some(client) = slf.client() {
        let object: Rc<dyn Object> = slf.clone();
        client.display().send_error(object, error.0, message);
    }
}

fn post_pool_error(slf: &Rc<WlShmPool>, error: WlShmPoolError, message: &str) {
    if let Some(client) = slf.client() {
        let object: Rc<dyn Object> = slf.clone();
        client.display().send_error(object, error.0, message);
    }
}
