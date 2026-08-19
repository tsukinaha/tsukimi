use std::{
    cell::RefCell,
    os::fd::OwnedFd,
    rc::Rc,
};

use wl_proxy::{
    object::{
        Object,
        ObjectCoreApi,
    },
    protocols::{
        drm::wl_drm::{
            WlDrm,
            WlDrmCapability,
            WlDrmError,
            WlDrmHandler,
        },
        wayland::wl_buffer::WlBuffer,
    },
};

use super::{
    SharedState,
    dmabuf::register_implicit_dmabuf,
};

pub(super) struct DrmHandler {
    pub(super) state: Rc<RefCell<SharedState>>,
}

impl WlDrmHandler for DrmHandler {
    fn handle_format(&mut self, slf: &Rc<WlDrm>, format: u32) {
        if self
            .state
            .borrow()
            .allowed_format_pairs
            .contains_implicit(format)
        {
            slf.send_format(format);
        }
    }

    fn handle_capabilities(&mut self, slf: &Rc<WlDrm>, mut value: u32) {
        if !self.state.borrow().allowed_format_pairs.has_implicit() {
            value &= !WlDrmCapability::PRIME.0;
        }
        slf.send_capabilities(value);
    }

    fn handle_create_buffer(
        &mut self, slf: &Rc<WlDrm>, id: &Rc<WlBuffer>, _name: u32, _width: i32, _height: i32,
        _stride: u32, _format: u32,
    ) {
        id.set_forward_to_server(false);
        send_drm_error(
            slf,
            WlDrmError::INVALID_NAME,
            "wl_drm GEM-name buffers are unsupported; PRIME capability is required",
        );
    }

    fn handle_create_planar_buffer(
        &mut self, slf: &Rc<WlDrm>, id: &Rc<WlBuffer>, _name: u32, _width: i32, _height: i32,
        _format: u32, _offset0: i32, _stride0: i32, _offset1: i32, _stride1: i32, _offset2: i32,
        _stride2: i32,
    ) {
        id.set_forward_to_server(false);
        send_drm_error(
            slf,
            WlDrmError::INVALID_NAME,
            "wl_drm GEM-name planar buffers are unsupported; PRIME capability is required",
        );
    }

    fn handle_create_prime_buffer(
        &mut self, slf: &Rc<WlDrm>, id: &Rc<WlBuffer>, fd: &Rc<OwnedFd>, width: i32, height: i32,
        format: u32, offset0: i32, stride0: i32, offset1: i32, stride1: i32, offset2: i32,
        stride2: i32,
    ) {
        id.set_forward_to_server(false);
        if !register_implicit_dmabuf(
            &self.state,
            id,
            fd,
            width,
            height,
            format,
            &[(offset0, stride0), (offset1, stride1), (offset2, stride2)],
        ) {
            send_drm_error(
                slf,
                WlDrmError::INVALID_FORMAT,
                "invalid or unsupported wl_drm PRIME buffer",
            );
        }
    }
}

fn send_drm_error(slf: &Rc<WlDrm>, error: WlDrmError, message: &str) {
    tracing::error!("wl-proxy-mpv: {message}");

    let Some(client) = slf.client() else {
        return;
    };

    let object: Rc<dyn Object> = slf.clone();
    client.display().send_error(object, error.0, message);
}
