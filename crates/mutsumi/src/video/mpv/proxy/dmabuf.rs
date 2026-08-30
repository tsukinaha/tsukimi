use std::{
    cell::RefCell,
    collections::HashSet,
    os::fd::{
        AsRawFd,
        FromRawFd,
        OwnedFd,
    },
    rc::Rc,
    sync::OnceLock,
};

use wl_proxy::{
    object::ObjectCoreApi,
    protocols::{
        linux_dmabuf_v1::{
            zwp_linux_buffer_params_v1::{
                ZwpLinuxBufferParamsV1,
                ZwpLinuxBufferParamsV1Flags,
                ZwpLinuxBufferParamsV1Handler,
            },
            zwp_linux_dmabuf_feedback_v1::{
                ZwpLinuxDmabufFeedbackV1,
                ZwpLinuxDmabufFeedbackV1Handler,
                ZwpLinuxDmabufFeedbackV1TrancheFlags,
            },
            zwp_linux_dmabuf_v1::{
                ZwpLinuxDmabufV1,
                ZwpLinuxDmabufV1Handler,
            },
        },
        wayland::{
            wl_buffer::{
                WlBuffer,
                WlBufferHandler,
            },
            wl_surface::WlSurface,
        },
    },
};

use super::{
    DmabufFrame,
    DmabufPlane,
    ProxyEvent,
    SharedState,
};

pub static ALLOWED_FORMAT_PAIRS: OnceLock<HashSet<(u32, u64)>> = OnceLock::new();

struct StoredPlane {
    fd: OwnedFd,
    offset: u32,
    stride: u32,
}

pub fn register_implicit_dmabuf(
    state: &Rc<RefCell<SharedState>>, buffer: &Rc<WlBuffer>, fd: &OwnedFd, width: i32, height: i32,
    format: u32, planes: &[(i32, i32)],
) {
    if width <= 0 || height <= 0 {
        tracing::error!("wl-proxy-mpv: invalid wl_drm buffer dimensions: {width}x{height}");
        return;
    }

    let planes = planes
        .iter()
        .filter(|(_, stride)| *stride > 0)
        .map(|(offset, stride)| {
            if *offset < 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "negative wl_drm plane offset",
                ));
            }

            Ok(StoredPlane {
                fd: fd.try_clone()?,
                offset: *offset as u32,
                stride: *stride as u32,
            })
        })
        .collect::<std::io::Result<Vec<_>>>();

    let Ok(planes) = planes else {
        tracing::error!("wl-proxy-mpv: failed to clone wl_drm PRIME fd");
        return;
    };
    if planes.is_empty() {
        tracing::error!("wl-proxy-mpv: wl_drm PRIME buffer has no planes");
        return;
    }

    buffer.set_forward_to_server(false);
    buffer.set_handler(WlBufferHandlerImpl {
        shared: Rc::clone(state),
    });

    state.borrow_mut().buffer_info.insert(
        buffer.unique_id(),
        BufferInfo {
            buffer: Rc::clone(buffer),
            planes,
            width: width as u32,
            height: height as u32,
            format,
            // wl_drm does not carry explicit modifier information.
            modifier: u64::MAX,
        },
    );
}

pub struct BufferInfo {
    pub buffer: Rc<WlBuffer>,
    planes: Vec<StoredPlane>,
    width: u32,
    height: u32,
    format: u32,
    modifier: u64,
}

impl BufferInfo {
    pub fn to_frame(
        &self, buffer_id: u64, event_tx: flume::Sender<ProxyEvent>,
    ) -> Option<DmabufFrame> {
        let planes = self
            .planes
            .iter()
            .map(|p| {
                p.fd.try_clone().map(|fd| DmabufPlane {
                    fd,
                    offset: p.offset,
                    stride: p.stride,
                })
            })
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| tracing::error!("wl-proxy-mpv: failed to clone dmabuf fd: {e}"))
            .ok()?;

        Some(DmabufFrame {
            width: self.width,
            height: self.height,
            format: self.format,
            modifier: self.modifier,
            planes,
            buffer_id,
            event_tx,
            #[cfg(feature = "profiling")]
            profile_frame_id: super::profiling::begin_frame(),
        })
    }
}

pub struct DmabufHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl ZwpLinuxDmabufV1Handler for DmabufHandler {
    fn handle_format(&mut self, slf: &Rc<ZwpLinuxDmabufV1>, format: u32) {
        let allowed = ALLOWED_FORMAT_PAIRS.get();
        if allowed.is_some_and(|pairs| pairs.iter().any(|(f, _)| *f == format)) {
            slf.send_format(format);
        }
    }

    fn handle_modifier(
        &mut self, slf: &Rc<ZwpLinuxDmabufV1>, format: u32, modifier_hi: u32, modifier_lo: u32,
    ) {
        let modifier = ((modifier_hi as u64) << 32) | (modifier_lo as u64);
        let allowed = ALLOWED_FORMAT_PAIRS.get();
        if allowed.is_some_and(|pairs| pairs.contains(&(format, modifier))) {
            slf.send_modifier(format, modifier_hi, modifier_lo);
        }
    }

    fn handle_create_params(
        &mut self, _slf: &Rc<ZwpLinuxDmabufV1>, params_id: &Rc<ZwpLinuxBufferParamsV1>,
    ) {
        params_id.set_forward_to_server(false);
        params_id.set_handler(BufferParamsHandler {
            state: Rc::clone(&self.state),
            planes: Vec::new(),
            modifier: None,
        });
    }

    fn handle_get_default_feedback(
        &mut self, slf: &Rc<ZwpLinuxDmabufV1>, id: &Rc<ZwpLinuxDmabufFeedbackV1>,
    ) {
        let allowed = ALLOWED_FORMAT_PAIRS.get().cloned().unwrap_or_default();
        id.set_handler(FeedbackHandler::new(allowed));
        slf.send_get_default_feedback(id);
    }

    fn handle_get_surface_feedback(
        &mut self, slf: &Rc<ZwpLinuxDmabufV1>, id: &Rc<ZwpLinuxDmabufFeedbackV1>,
        _surface: &Rc<WlSurface>,
    ) {
        let allowed = ALLOWED_FORMAT_PAIRS.get().cloned().unwrap_or_default();
        id.set_handler(FeedbackHandler::new(allowed));
        slf.send_get_default_feedback(id);
    }
}

struct FeedbackHandler {
    allowed: HashSet<(u32, u64)>,
    index_map: Vec<Option<u16>>,
    pending_device: Option<Vec<u8>>,
    pending_flags: Option<ZwpLinuxDmabufFeedbackV1TrancheFlags>,
    pending_formats: Vec<u16>,
}

impl FeedbackHandler {
    fn new(allowed: HashSet<(u32, u64)>) -> Self {
        Self {
            allowed,
            index_map: Vec::new(),
            pending_device: None,
            pending_flags: None,
            pending_formats: Vec::new(),
        }
    }
}

impl ZwpLinuxDmabufFeedbackV1Handler for FeedbackHandler {
    fn handle_format_table(
        &mut self, slf: &Rc<ZwpLinuxDmabufFeedbackV1>, fd: &Rc<OwnedFd>, size: u32,
    ) {
        let num_entries = size as usize / 16;
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size as usize,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                fd.as_raw_fd(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            slf.send_format_table(fd, size);
            return;
        }

        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, size as usize) };
        let mut new_table: Vec<u8> = Vec::new();
        self.index_map = vec![None; num_entries];
        let mut new_index: u16 = 0;

        for i in 0..num_entries {
            let base = i * 16;
            let format = u32::from_ne_bytes(bytes[base..base + 4].try_into().unwrap());
            let modifier = u64::from_ne_bytes(bytes[base + 8..base + 16].try_into().unwrap());
            if self.allowed.contains(&(format, modifier)) {
                self.index_map[i] = Some(new_index);
                new_index = new_index.saturating_add(1);
                new_table.extend_from_slice(&bytes[base..base + 16]);
            }
        }

        unsafe { libc::munmap(ptr, size as usize) };

        let memfd = unsafe { libc::memfd_create(c"dmabuf-fb".as_ptr() as *const libc::c_char, 0) };
        if memfd < 0 {
            return;
        }
        unsafe {
            libc::write(
                memfd,
                new_table.as_ptr() as *const libc::c_void,
                new_table.len(),
            )
        };
        let new_fd = Rc::new(unsafe { OwnedFd::from_raw_fd(memfd) });
        slf.send_format_table(&new_fd, new_table.len() as u32);
    }

    fn handle_main_device(&mut self, slf: &Rc<ZwpLinuxDmabufFeedbackV1>, device: &[u8]) {
        slf.send_main_device(device);
    }

    fn handle_tranche_target_device(&mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>, device: &[u8]) {
        self.pending_device = Some(device.to_vec());
        self.pending_flags = None;
        self.pending_formats.clear();
    }

    fn handle_tranche_flags(
        &mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>, flags: ZwpLinuxDmabufFeedbackV1TrancheFlags,
    ) {
        self.pending_flags = Some(flags);
    }

    fn handle_tranche_formats(&mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>, indices: &[u8]) {
        for chunk in indices.chunks_exact(2) {
            let old = u16::from_ne_bytes([chunk[0], chunk[1]]);
            if let Some(Some(new)) = self.index_map.get(old as usize) {
                self.pending_formats.push(*new);
            }
        }
    }

    fn handle_tranche_done(&mut self, slf: &Rc<ZwpLinuxDmabufFeedbackV1>) {
        if self.pending_formats.is_empty() {
            self.pending_device = None;
            self.pending_flags = None;
            return;
        }
        if let Some(device) = self.pending_device.take() {
            slf.send_tranche_target_device(&device);
        }
        slf.send_tranche_flags(self.pending_flags.take().unwrap_or_default());
        let bytes: Vec<u8> = self
            .pending_formats
            .drain(..)
            .flat_map(|i| i.to_ne_bytes())
            .collect();
        slf.send_tranche_formats(&bytes);
        slf.send_tranche_done();
    }

    fn handle_done(&mut self, slf: &Rc<ZwpLinuxDmabufFeedbackV1>) {
        slf.send_done();
    }
}

struct BufferParamsHandler {
    state: Rc<RefCell<SharedState>>,
    planes: Vec<Option<StoredPlane>>,
    modifier: Option<u64>,
}

impl BufferParamsHandler {
    fn register_buffer(
        &mut self, buffer: &Rc<WlBuffer>, width: i32, height: i32, format: u32,
    ) -> bool {
        let Some(planes): Option<Vec<_>> = std::mem::take(&mut self.planes).into_iter().collect()
        else {
            tracing::error!("wl-proxy-mpv: dmabuf has missing planes");
            return false;
        };
        if planes.is_empty() || width <= 0 || height <= 0 {
            tracing::error!(
                "wl-proxy-mpv: invalid dmabuf dimensions or plane count: {width}x{height}, {} planes",
                planes.len()
            );
            return false;
        }

        buffer.set_forward_to_server(false);
        buffer.set_handler(WlBufferHandlerImpl {
            shared: Rc::clone(&self.state),
        });

        self.state.borrow_mut().buffer_info.insert(
            buffer.unique_id(),
            BufferInfo {
                buffer: Rc::clone(buffer),
                planes,
                width: width as u32,
                height: height as u32,
                format,
                modifier: self.modifier.take().unwrap_or(0),
            },
        );
        true
    }
}

impl ZwpLinuxBufferParamsV1Handler for BufferParamsHandler {
    fn handle_destroy(&mut self, slf: &Rc<ZwpLinuxBufferParamsV1>) {
        slf.delete_id();
    }

    fn handle_add(
        &mut self, _slf: &Rc<ZwpLinuxBufferParamsV1>, fd: &Rc<OwnedFd>, plane_idx: u32,
        offset: u32, stride: u32, modifier_hi: u32, modifier_lo: u32,
    ) {
        let plane_idx = plane_idx as usize;
        if plane_idx >= 4 {
            tracing::error!("wl-proxy-mpv: invalid dmabuf plane index {plane_idx}");
            return;
        }
        if self.planes.len() <= plane_idx {
            self.planes.resize_with(plane_idx + 1, || None);
        }
        if self.planes[plane_idx].is_some() {
            tracing::error!("wl-proxy-mpv: dmabuf plane {plane_idx} was already set");
            return;
        }

        let modifier = ((modifier_hi as u64) << 32) | (modifier_lo as u64);
        if self.modifier.is_some_and(|current| current != modifier) {
            tracing::error!("wl-proxy-mpv: dmabuf planes use different modifiers");
            return;
        }
        let dup_fd = match fd.try_clone() {
            Ok(fd) => fd,
            Err(e) => {
                tracing::error!("wl-proxy-mpv: failed to clone dmabuf plane fd: {e}");
                return;
            }
        };

        self.modifier = Some(modifier);
        self.planes[plane_idx] = Some(StoredPlane {
            fd: dup_fd,
            offset,
            stride,
        });
    }

    fn handle_create(
        &mut self, slf: &Rc<ZwpLinuxBufferParamsV1>, width: i32, height: i32, format: u32,
        _flags: ZwpLinuxBufferParamsV1Flags,
    ) {
        let valid = width > 0
            && height > 0
            && !self.planes.is_empty()
            && self.planes.iter().all(Option::is_some);
        if !valid {
            tracing::error!(
                "wl-proxy-mpv: invalid asynchronous dmabuf: {width}x{height}, {} planes",
                self.planes.len()
            );
            slf.send_failed();
            return;
        }

        let buffer = slf.new_send_created();
        let registered = self.register_buffer(&buffer, width, height, format);
        debug_assert!(registered);
    }

    fn handle_create_immed(
        &mut self, _slf: &Rc<ZwpLinuxBufferParamsV1>, buffer_id: &Rc<WlBuffer>, width: i32,
        height: i32, format: u32, _flags: ZwpLinuxBufferParamsV1Flags,
    ) {
        self.register_buffer(buffer_id, width, height, format);
    }
}

struct WlBufferHandlerImpl {
    shared: Rc<RefCell<SharedState>>,
}

impl WlBufferHandler for WlBufferHandlerImpl {
    fn handle_destroy(&mut self, slf: &Rc<WlBuffer>) {
        self.shared
            .borrow_mut()
            .buffer_info
            .remove(&slf.unique_id());

        slf.delete_id();
    }
}
