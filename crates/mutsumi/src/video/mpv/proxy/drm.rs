use std::{cell::RefCell, os::fd::OwnedFd, rc::Rc};

use wl_proxy::protocols::{
    drm::wl_drm::{WlDrm, WlDrmHandler},
    wayland::wl_buffer::WlBuffer,
};

use super::{SharedState, dmabuf::register_implicit_dmabuf};

pub struct DrmHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl WlDrmHandler for DrmHandler {
    fn handle_create_buffer(
        &mut self,
        _slf: &Rc<WlDrm>,
        _id: &Rc<WlBuffer>,
        _name: u32,
        _width: i32,
        _height: i32,
        _stride: u32,
        _format: u32,
    ) {
        tracing::warn!(
            "wl-proxy-mpv: wl_drm GEM-name buffers are unsupported; PRIME capability is required"
        );
    }

    fn handle_create_planar_buffer(
        &mut self,
        _slf: &Rc<WlDrm>,
        _id: &Rc<WlBuffer>,
        _name: u32,
        _width: i32,
        _height: i32,
        _format: u32,
        _offset0: i32,
        _stride0: i32,
        _offset1: i32,
        _stride1: i32,
        _offset2: i32,
        _stride2: i32,
    ) {
        tracing::warn!(
            "wl-proxy-mpv: wl_drm GEM-name planar buffers are unsupported; PRIME capability is required"
        );
    }

    fn handle_create_prime_buffer(
        &mut self,
        _slf: &Rc<WlDrm>,
        id: &Rc<WlBuffer>,
        fd: &Rc<OwnedFd>,
        width: i32,
        height: i32,
        format: u32,
        offset0: i32,
        stride0: i32,
        offset1: i32,
        stride1: i32,
        offset2: i32,
        stride2: i32,
    ) {
        register_implicit_dmabuf(
            &self.state,
            id,
            fd,
            width,
            height,
            format,
            &[(offset0, stride0), (offset1, stride1), (offset2, stride2)],
        );
    }
}
