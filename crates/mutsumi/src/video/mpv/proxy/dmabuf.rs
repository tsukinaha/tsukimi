use std::{
    cell::RefCell,
    collections::HashSet,
    os::fd::OwnedFd,
    rc::Rc,
};

use wl_proxy::{
    object::{
        Object,
        ObjectCoreApi,
    },
    protocols::{
        linux_dmabuf_v1::{
            zwp_linux_buffer_params_v1::{
                ZwpLinuxBufferParamsV1,
                ZwpLinuxBufferParamsV1Error,
                ZwpLinuxBufferParamsV1Flags,
                ZwpLinuxBufferParamsV1Handler,
            },
            zwp_linux_dmabuf_v1::{
                ZwpLinuxDmabufV1,
                ZwpLinuxDmabufV1Handler,
            },
        },
        wayland::wl_buffer::{
            WlBuffer,
            WlBufferHandler,
        },
    },
};

use super::{
    BufferUseToken,
    DmabufFrame,
    DmabufPlane,
    SharedState,
    surface_tree::BufferMetadata,
};

pub(crate) const DRM_FORMAT_MOD_INVALID: u64 = 0x00ff_ffff_ffff_ffff;

#[derive(Debug)]
pub(crate) struct AllowedFormatPairs {
    pairs: Vec<(u32, u64)>,
    set: HashSet<(u32, u64)>,
}

impl AllowedFormatPairs {
    pub(crate) fn new(mut pairs: Vec<(u32, u64)>) -> Self {
        pairs.sort_unstable();
        pairs.dedup();
        let set = pairs.iter().copied().collect();
        Self { pairs, set }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub(crate) fn contains(&self, format: u32, modifier: u64) -> bool {
        self.set.contains(&(format, modifier))
    }

    fn pairs(&self) -> &[(u32, u64)] {
        &self.pairs
    }
}

struct StoredPlane {
    fd: OwnedFd,
    offset: u32,
    stride: u32,
}

pub struct BufferInfo {
    pub buffer: Rc<WlBuffer>,
    planes: Vec<StoredPlane>,
    width: u32,
    height: u32,
    format: u32,
    modifier: u64,
}

pub(crate) struct FrameData {
    width: u32,
    height: u32,
    format: u32,
    modifier: u64,
    planes: Vec<DmabufPlane>,
}

impl FrameData {
    pub(crate) fn into_frame(self, use_token: BufferUseToken) -> DmabufFrame {
        DmabufFrame {
            width: self.width,
            height: self.height,
            format: self.format,
            modifier: self.modifier,
            planes: self.planes,
            _use_token: use_token,
            #[cfg(feature = "profiling")]
            profile_frame_id: super::profiling::begin_frame(),
        }
    }
}

impl BufferInfo {
    pub fn metadata(&self, buffer_id: u64) -> BufferMetadata {
        BufferMetadata {
            buffer_id,
            width: self.width,
            height: self.height,
        }
    }

    pub fn frame_data(&self) -> Option<FrameData> {
        let planes = self
            .planes
            .iter()
            .map(|plane| {
                plane.fd.try_clone().map(|fd| DmabufPlane {
                    fd,
                    offset: plane.offset,
                    stride: plane.stride,
                })
            })
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|error| tracing::error!("wl-proxy-mpv: failed to clone dmabuf fd: {error}"))
            .ok()?;

        Some(FrameData {
            width: self.width,
            height: self.height,
            format: self.format,
            modifier: self.modifier,
            planes,
        })
    }
}

pub struct DmabufHandler {
    pub state: Rc<RefCell<SharedState>>,
}

pub(crate) fn advertise_formats(dmabuf: &ZwpLinuxDmabufV1, allowed: &AllowedFormatPairs) {
    let mut previous_format = None;
    for &(format, modifier) in allowed.pairs() {
        if previous_format != Some(format) {
            dmabuf.send_format(format);
            previous_format = Some(format);
        }
        let (modifier_hi, modifier_lo) = split_modifier(modifier);
        dmabuf.send_modifier(format, modifier_hi, modifier_lo);
    }
}

impl ZwpLinuxDmabufV1Handler for DmabufHandler {
    fn handle_destroy(&mut self, slf: &Rc<ZwpLinuxDmabufV1>) {
        slf.delete_id();
    }

    fn handle_create_params(
        &mut self, _slf: &Rc<ZwpLinuxDmabufV1>, params_id: &Rc<ZwpLinuxBufferParamsV1>,
    ) {
        params_id.set_forward_to_server(false);
        params_id.set_handler(BufferParamsHandler {
            state: Rc::clone(&self.state),
            planes: Vec::new(),
            modifier: None,
            used: false,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidationError {
    Incomplete,
    InvalidDimensions,
    InvalidFormat,
}

struct BufferParamsHandler {
    state: Rc<RefCell<SharedState>>,
    planes: Vec<Option<StoredPlane>>,
    modifier: Option<u64>,
    used: bool,
}

impl BufferParamsHandler {
    fn validate(&self, width: i32, height: i32, format: u32) -> Result<u64, ValidationError> {
        if width <= 0 || height <= 0 {
            return Err(ValidationError::InvalidDimensions);
        }
        if self.planes.is_empty() || self.planes.iter().any(Option::is_none) {
            return Err(ValidationError::Incomplete);
        }
        let modifier = self.modifier.unwrap_or(DRM_FORMAT_MOD_INVALID);
        if !self
            .state
            .borrow()
            .allowed_format_pairs
            .contains(format, modifier)
        {
            return Err(ValidationError::InvalidFormat);
        }
        Ok(modifier)
    }

    fn register_buffer(
        &mut self, buffer: &Rc<WlBuffer>, width: i32, height: i32, format: u32, modifier: u64,
    ) {
        let planes = std::mem::take(&mut self.planes)
            .into_iter()
            .map(Option::unwrap)
            .collect();
        self.modifier.take();

        buffer.set_forward_to_server(false);
        buffer.set_handler(WlBufferHandlerImpl {
            shared: Rc::clone(&self.state),
        });
        let buffer_id = buffer.unique_id();
        let mut state = self.state.borrow_mut();
        state.register_buffer(buffer_id);
        state.buffer_info.insert(
            buffer_id,
            BufferInfo {
                buffer: Rc::clone(buffer),
                planes,
                width: width as u32,
                height: height as u32,
                format,
                modifier,
            },
        );
    }

    fn log_validation_error(&self, error: ValidationError, width: i32, height: i32, format: u32) {
        tracing::error!(
            ?error,
            width,
            height,
            format,
            modifier = self.modifier.unwrap_or(DRM_FORMAT_MOD_INVALID),
            planes = self.planes.len(),
            "wl-proxy-mpv: rejected invalid dmabuf"
        );
    }
}

impl ZwpLinuxBufferParamsV1Handler for BufferParamsHandler {
    fn handle_destroy(&mut self, slf: &Rc<ZwpLinuxBufferParamsV1>) {
        slf.delete_id();
    }

    fn handle_add(
        &mut self, slf: &Rc<ZwpLinuxBufferParamsV1>, fd: &Rc<OwnedFd>, plane_idx: u32, offset: u32,
        stride: u32, modifier_hi: u32, modifier_lo: u32,
    ) {
        let plane_idx = plane_idx as usize;
        if self.used {
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::ALREADY_USED,
                "DMA-BUF params object was already used",
            );
            return;
        }
        if plane_idx >= 4 {
            tracing::error!("wl-proxy-mpv: invalid dmabuf plane index {plane_idx}");
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::PLANE_IDX,
                "invalid DMA-BUF plane index",
            );
            return;
        }
        if stride == 0 {
            tracing::error!("wl-proxy-mpv: dmabuf plane {plane_idx} has zero stride");
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::OUT_OF_BOUNDS,
                "DMA-BUF plane has zero stride",
            );
            return;
        }
        if self.planes.len() <= plane_idx {
            self.planes.resize_with(plane_idx + 1, || None);
        }
        if self.planes[plane_idx].is_some() {
            tracing::error!("wl-proxy-mpv: dmabuf plane {plane_idx} was already set");
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::PLANE_SET,
                "DMA-BUF plane was already set",
            );
            return;
        }

        let modifier = combine_modifier(modifier_hi, modifier_lo);
        if self.modifier.is_some_and(|current| current != modifier) {
            tracing::error!("wl-proxy-mpv: dmabuf planes use different modifiers");
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::INVALID_FORMAT,
                "DMA-BUF planes use different modifiers",
            );
            return;
        }
        let dup_fd = match fd.try_clone() {
            Ok(fd) => fd,
            Err(error) => {
                tracing::error!("wl-proxy-mpv: failed to clone dmabuf plane fd: {error}");
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
        flags: ZwpLinuxBufferParamsV1Flags,
    ) {
        if self.used {
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::ALREADY_USED,
                "DMA-BUF params object was already used",
            );
            return;
        }
        self.used = true;
        if !flags.is_empty() {
            tracing::error!(?flags, "wl-proxy-mpv: unsupported DMA-BUF flags");
            slf.send_failed();
            return;
        }
        let modifier = match self.validate(width, height, format) {
            Ok(modifier) => modifier,
            Err(error) => {
                self.log_validation_error(error, width, height, format);
                slf.send_failed();
                return;
            }
        };

        let buffer = slf.new_send_created();
        self.register_buffer(&buffer, width, height, format, modifier);
    }

    fn handle_create_immed(
        &mut self, slf: &Rc<ZwpLinuxBufferParamsV1>, buffer_id: &Rc<WlBuffer>, width: i32,
        height: i32, format: u32, flags: ZwpLinuxBufferParamsV1Flags,
    ) {
        if self.used {
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::ALREADY_USED,
                "DMA-BUF params object was already used",
            );
            return;
        }
        self.used = true;
        if !flags.is_empty() {
            send_params_error(
                slf,
                ZwpLinuxBufferParamsV1Error::INVALID_FORMAT,
                "unsupported DMA-BUF flags",
            );
            return;
        }
        match self.validate(width, height, format) {
            Ok(modifier) => {
                self.register_buffer(buffer_id, width, height, format, modifier);
            }
            Err(error) => {
                self.log_validation_error(error, width, height, format);
                send_validation_error(slf, error);
            }
        }
    }
}

fn send_validation_error(slf: &Rc<ZwpLinuxBufferParamsV1>, error: ValidationError) {
    let code = match error {
        ValidationError::Incomplete => ZwpLinuxBufferParamsV1Error::INCOMPLETE,
        ValidationError::InvalidDimensions => ZwpLinuxBufferParamsV1Error::INVALID_DIMENSIONS,
        ValidationError::InvalidFormat => ZwpLinuxBufferParamsV1Error::INVALID_FORMAT,
    };
    send_params_error(slf, code, "invalid DMA-BUF parameters");
}

fn send_params_error(
    slf: &Rc<ZwpLinuxBufferParamsV1>, error: ZwpLinuxBufferParamsV1Error, message: &str,
) {
    if let Some(client) = slf.client() {
        let object: Rc<dyn Object> = slf.clone();
        client.display().send_error(object, error.0, message);
    }
}

struct WlBufferHandlerImpl {
    shared: Rc<RefCell<SharedState>>,
}

impl WlBufferHandler for WlBufferHandlerImpl {
    fn handle_destroy(&mut self, slf: &Rc<WlBuffer>) {
        self.shared.borrow_mut().buffer_destroyed(slf.unique_id());
        slf.delete_id();
    }
}

fn combine_modifier(modifier_hi: u32, modifier_lo: u32) -> u64 {
    (u64::from(modifier_hi) << 32) | u64::from(modifier_lo)
}

fn split_modifier(modifier: u64) -> (u32, u32) {
    ((modifier >> 32) as u32, modifier as u32)
}
