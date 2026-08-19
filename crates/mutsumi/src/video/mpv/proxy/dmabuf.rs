use std::{
    cell::RefCell,
    collections::HashSet,
    os::fd::{
        AsRawFd,
        FromRawFd,
        OwnedFd,
    },
    rc::{
        Rc,
        Weak,
    },
    sync::Arc,
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

    pub(crate) fn contains_implicit(&self, format: u32) -> bool {
        self.contains(format, DRM_FORMAT_MOD_INVALID)
    }

    pub(crate) fn has_implicit(&self) -> bool {
        self.pairs
            .iter()
            .any(|(_, modifier)| *modifier == DRM_FORMAT_MOD_INVALID)
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

pub(super) fn register_implicit_dmabuf(
    state: &Rc<RefCell<SharedState>>, buffer: &Rc<WlBuffer>, fd: &OwnedFd, width: i32, height: i32,
    format: u32, planes: &[(i32, i32)],
) -> bool {
    if width <= 0 || height <= 0 {
        tracing::error!("wl-proxy-mpv: invalid wl_drm buffer dimensions: {width}x{height}");
        return false;
    }
    if !state
        .borrow()
        .allowed_format_pairs
        .contains_implicit(format)
    {
        tracing::error!(format, "wl-proxy-mpv: unsupported implicit wl_drm format");
        return false;
    }

    let mut stored_planes = Vec::new();
    let mut found_unused_plane = false;
    for &(offset, stride) in planes {
        if stride == 0 {
            found_unused_plane = true;
            continue;
        }
        if offset < 0 || stride < 0 || found_unused_plane {
            tracing::error!("wl-proxy-mpv: invalid wl_drm PRIME plane layout");
            return false;
        }
        let Ok(fd) = fd.try_clone() else {
            tracing::error!("wl-proxy-mpv: failed to clone wl_drm PRIME fd");
            return false;
        };
        stored_planes.push(StoredPlane {
            fd,
            offset: offset as u32,
            stride: stride as u32,
        });
    }
    if stored_planes.is_empty() {
        tracing::error!("wl-proxy-mpv: wl_drm PRIME buffer has no planes");
        return false;
    }

    buffer.set_forward_to_server(false);
    buffer.set_handler(WlBufferHandlerImpl {
        shared: Rc::clone(state),
    });
    let buffer_id = buffer.unique_id();
    let mut state = state.borrow_mut();
    state.register_buffer(buffer_id);
    state.buffer_info.insert(
        buffer_id,
        BufferInfo {
            buffer: Rc::clone(buffer),
            planes: stored_planes,
            width: width as u32,
            height: height as u32,
            format,
            modifier: DRM_FORMAT_MOD_INVALID,
        },
    );
    true
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
    pub forward_to_upstream: bool,
}

pub(crate) fn advertise_formats(dmabuf: &ZwpLinuxDmabufV1, allowed: &AllowedFormatPairs) {
    let mut previous_format = None;
    for &(format, modifier) in allowed.pairs() {
        if previous_format != Some(format) {
            if allowed.contains_implicit(format) {
                dmabuf.send_format(format);
            }
            previous_format = Some(format);
        }
        let (modifier_hi, modifier_lo) = split_modifier(modifier);
        dmabuf.send_modifier(format, modifier_hi, modifier_lo);
    }
}

impl ZwpLinuxDmabufV1Handler for DmabufHandler {
    fn handle_format(&mut self, slf: &Rc<ZwpLinuxDmabufV1>, format: u32) {
        if self
            .state
            .borrow()
            .allowed_format_pairs
            .contains_implicit(format)
        {
            slf.send_format(format);
        }
    }

    fn handle_modifier(
        &mut self, slf: &Rc<ZwpLinuxDmabufV1>, format: u32, modifier_hi: u32, modifier_lo: u32,
    ) {
        let modifier = combine_modifier(modifier_hi, modifier_lo);
        if self
            .state
            .borrow()
            .allowed_format_pairs
            .contains(format, modifier)
        {
            slf.send_modifier(format, modifier_hi, modifier_lo);
        }
    }

    fn handle_destroy(&mut self, slf: &Rc<ZwpLinuxDmabufV1>) {
        if self.forward_to_upstream {
            slf.send_destroy();
        } else {
            slf.delete_id();
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
            used: false,
        });
    }

    fn handle_get_default_feedback(
        &mut self, slf: &Rc<ZwpLinuxDmabufV1>, feedback: &Rc<ZwpLinuxDmabufFeedbackV1>,
    ) {
        let allowed = Arc::clone(&self.state.borrow().allowed_format_pairs);
        feedback.set_handler(FeedbackHandler::new(allowed, None));
        slf.send_get_default_feedback(feedback);
    }

    fn handle_get_surface_feedback(
        &mut self, slf: &Rc<ZwpLinuxDmabufV1>, feedback: &Rc<ZwpLinuxDmabufFeedbackV1>,
        surface: &Rc<WlSurface>,
    ) {
        let allowed = Arc::clone(&self.state.borrow().allowed_format_pairs);
        let surface = (Rc::downgrade(&self.state), surface.unique_id());
        feedback.set_handler(FeedbackHandler::new(allowed, Some(surface)));
        // Synthetic surfaces have no upstream counterpart. The compositor's
        // default feedback still carries the render device Mesa needs.
        slf.send_get_default_feedback(feedback);
    }
}

struct FeedbackTranche {
    device: Vec<u8>,
    flags: ZwpLinuxDmabufFeedbackV1TrancheFlags,
    formats: Vec<u16>,
}

fn select_main_device(
    upstream_main_device: Option<&[u8]>, tranches: &[FeedbackTranche],
) -> Option<Vec<u8>> {
    upstream_main_device
        .filter(|main_device| {
            tranches
                .iter()
                .any(|tranche| tranche.device.as_slice() == *main_device)
        })
        .map(<[u8]>::to_vec)
        .or_else(|| {
            tranches
                .iter()
                .find(|tranche| {
                    tranche
                        .flags
                        .contains(ZwpLinuxDmabufFeedbackV1TrancheFlags::SAMPLING)
                })
                .map(|tranche| tranche.device.clone())
        })
        .or_else(|| tranches.first().map(|tranche| tranche.device.clone()))
}

struct FeedbackHandler {
    allowed: Arc<AllowedFormatPairs>,
    index_map: Vec<Option<u16>>,
    format_table: Option<(Rc<OwnedFd>, u32)>,
    main_device: Option<Vec<u8>>,
    tranches: Vec<FeedbackTranche>,
    pending_device: Option<Vec<u8>>,
    pending_flags: Option<ZwpLinuxDmabufFeedbackV1TrancheFlags>,
    pending_formats: Vec<u16>,
    failed: bool,
    surface: Option<(Weak<RefCell<SharedState>>, u64)>,
}

impl FeedbackHandler {
    fn new(
        allowed: Arc<AllowedFormatPairs>, surface: Option<(Weak<RefCell<SharedState>>, u64)>,
    ) -> Self {
        Self {
            allowed,
            index_map: Vec::new(),
            format_table: None,
            main_device: None,
            tranches: Vec::new(),
            pending_device: None,
            pending_flags: None,
            pending_formats: Vec::new(),
            failed: false,
            surface,
        }
    }

    fn surface_is_inert(&mut self) -> bool {
        let inert = self.surface.as_ref().is_some_and(|(state, surface_id)| {
            state
                .upgrade()
                .is_none_or(|state| !state.borrow().surfaces.contains_key(surface_id))
        });
        if inert {
            self.clear_update();
        }
        inert
    }

    fn clear_update(&mut self) {
        self.index_map.clear();
        self.format_table = None;
        self.main_device = None;
        self.tranches.clear();
        self.pending_device = None;
        self.pending_flags = None;
        self.pending_formats.clear();
    }

    fn fail(&mut self, slf: &Rc<ZwpLinuxDmabufFeedbackV1>, message: &str) {
        if self.failed {
            return;
        }
        self.failed = true;
        self.clear_update();
        tracing::error!("wl-proxy-mpv: {message}");
        if let Some(client) = slf.client() {
            let object: Rc<dyn Object> = slf.clone();
            client.display().send_error(object, 0, message);
        }
    }
}

impl ZwpLinuxDmabufFeedbackV1Handler for FeedbackHandler {
    fn handle_format_table(
        &mut self, slf: &Rc<ZwpLinuxDmabufFeedbackV1>, fd: &Rc<OwnedFd>, size: u32,
    ) {
        if self.failed || self.surface_is_inert() {
            return;
        }
        self.clear_update();
        if size == 0 || size % 16 != 0 {
            self.fail(slf, "invalid dmabuf feedback format table size");
            return;
        }

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
            self.fail(slf, "failed to map dmabuf feedback format table");
            return;
        }

        let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), size as usize) };
        let mut filtered_table = Vec::new();
        self.index_map = vec![None; num_entries];

        for index in 0..num_entries {
            let base = index * 16;
            let format = u32::from_ne_bytes(bytes[base..base + 4].try_into().unwrap());
            let modifier = u64::from_ne_bytes(bytes[base + 8..base + 16].try_into().unwrap());
            if self.allowed.contains(format, modifier) {
                let filtered_index = filtered_table.len() / 16;
                if let Ok(filtered_index) = u16::try_from(filtered_index) {
                    self.index_map[index] = Some(filtered_index);
                    filtered_table.extend_from_slice(&bytes[base..base + 16]);
                }
            }
        }

        unsafe { libc::munmap(ptr, size as usize) };

        let memfd =
            unsafe { libc::memfd_create(c"mutsumi-dmabuf-feedback".as_ptr(), libc::MFD_CLOEXEC) };
        if memfd < 0 {
            self.fail(slf, "failed to create filtered dmabuf feedback table");
            return;
        }
        let filtered_fd = unsafe { OwnedFd::from_raw_fd(memfd) };
        let written = unsafe {
            libc::write(
                filtered_fd.as_raw_fd(),
                filtered_table.as_ptr().cast(),
                filtered_table.len(),
            )
        };
        if written < 0 || written as usize != filtered_table.len() {
            self.fail(slf, "failed to write filtered dmabuf feedback table");
            return;
        }

        self.format_table = Some((Rc::new(filtered_fd), filtered_table.len() as u32));
    }

    fn handle_main_device(&mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>, device: &[u8]) {
        if !self.failed && !self.surface_is_inert() {
            self.main_device = Some(device.to_vec());
        }
    }

    fn handle_tranche_target_device(&mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>, device: &[u8]) {
        if self.failed || self.surface_is_inert() {
            return;
        }
        self.pending_device = Some(device.to_vec());
        self.pending_flags = None;
        self.pending_formats.clear();
    }

    fn handle_tranche_flags(
        &mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>,
        mut flags: ZwpLinuxDmabufFeedbackV1TrancheFlags,
    ) {
        if !self.failed && !self.surface_is_inert() {
            flags.remove(ZwpLinuxDmabufFeedbackV1TrancheFlags::SCANOUT);
            self.pending_flags = Some(flags);
        }
    }

    fn handle_tranche_formats(&mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>, indices: &[u8]) {
        if self.failed || self.surface_is_inert() {
            return;
        }
        for bytes in indices.chunks_exact(2) {
            let upstream_index = u16::from_ne_bytes([bytes[0], bytes[1]]);
            if let Some(Some(filtered_index)) = self.index_map.get(upstream_index as usize) {
                self.pending_formats.push(*filtered_index);
            }
        }
    }

    fn handle_tranche_done(&mut self, _slf: &Rc<ZwpLinuxDmabufFeedbackV1>) {
        if self.failed || self.surface_is_inert() {
            return;
        }
        let device = self.pending_device.take();
        let formats = std::mem::take(&mut self.pending_formats);
        let flags = self.pending_flags.take().unwrap_or_default();
        if let Some(device) = device
            && !formats.is_empty()
        {
            self.tranches.push(FeedbackTranche {
                device,
                flags,
                formats,
            });
        }
    }

    fn handle_done(&mut self, slf: &Rc<ZwpLinuxDmabufFeedbackV1>) {
        if self.failed || self.surface_is_inert() {
            return;
        }
        let upstream_main_device = self.main_device.take();
        let main_device = select_main_device(upstream_main_device.as_deref(), &self.tranches);
        let valid = self
            .format_table
            .as_ref()
            .is_some_and(|(_, size)| *size > 0)
            && main_device.is_some();
        if !valid {
            self.fail(slf, "filtered dmabuf feedback has no usable device tranche");
            return;
        }

        let (format_table, size) = self.format_table.take().unwrap();
        let main_device = main_device.unwrap();
        if upstream_main_device.as_ref() != Some(&main_device) {
            tracing::debug!(
                "wl-proxy-mpv: selected a GDK-compatible dmabuf tranche as the synthetic main device"
            );
        }
        slf.send_format_table(&format_table, size);
        slf.send_main_device(&main_device);
        for tranche in self.tranches.drain(..) {
            slf.send_tranche_target_device(&tranche.device);
            slf.send_tranche_flags(tranche.flags);
            let indices: Vec<u8> = tranche
                .formats
                .into_iter()
                .flat_map(u16::to_ne_bytes)
                .collect();
            slf.send_tranche_formats(&indices);
            slf.send_tranche_done();
        }
        slf.send_done();
        self.index_map.clear();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tranche(device: u8, flags: ZwpLinuxDmabufFeedbackV1TrancheFlags) -> FeedbackTranche {
        FeedbackTranche {
            device: vec![device],
            flags,
            formats: vec![0],
        }
    }

    #[test]
    fn implicit_formats_require_the_invalid_modifier_marker() {
        let formats = AllowedFormatPairs::new(vec![(1, 2), (1, DRM_FORMAT_MOD_INVALID)]);
        assert!(formats.contains_implicit(1));
        assert!(!formats.contains_implicit(2));
    }

    #[test]
    fn main_device_selection_prefers_a_retained_upstream_main_device() {
        let tranches = vec![
            tranche(1, ZwpLinuxDmabufFeedbackV1TrancheFlags::empty()),
            tranche(2, ZwpLinuxDmabufFeedbackV1TrancheFlags::SAMPLING),
        ];
        assert_eq!(select_main_device(Some(&[1]), &tranches), Some(vec![1]));
    }

    #[test]
    fn main_device_selection_falls_back_to_a_sampling_device() {
        let tranches = vec![
            tranche(1, ZwpLinuxDmabufFeedbackV1TrancheFlags::empty()),
            tranche(2, ZwpLinuxDmabufFeedbackV1TrancheFlags::SAMPLING),
        ];
        assert_eq!(select_main_device(Some(&[3]), &tranches), Some(vec![2]));
    }
}
