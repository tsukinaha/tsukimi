use std::{
    cell::RefCell,
    rc::Rc,
};

use wl_proxy::{
    object::ObjectCoreApi,
    protocols::{
        wayland::wl_surface::WlSurface,
        xdg_shell::{
            xdg_surface::{
                XdgSurface,
                XdgSurfaceHandler,
            },
            xdg_toplevel::{
                XdgToplevel,
                XdgToplevelHandler,
            },
            xdg_wm_base::{
                XdgWmBase,
                XdgWmBaseHandler,
            },
        },
    },
};

use super::SharedState;

pub struct ToplevelEntry {
    pub xdg_surface: Rc<XdgSurface>,
    pub toplevel: Rc<XdgToplevel>,
}

pub struct WmBaseHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl XdgWmBaseHandler for WmBaseHandler {
    fn handle_destroy(&mut self, slf: &Rc<XdgWmBase>) {
        slf.delete_id();
    }

    fn handle_get_xdg_surface(
        &mut self, _slf: &Rc<XdgWmBase>, id: &Rc<XdgSurface>, _surface: &Rc<WlSurface>,
    ) {
        id.set_forward_to_server(false);
        id.set_handler(XdgSurfaceHandlerImpl {
            state: Rc::clone(&self.state),
        });
    }

    fn handle_pong(&mut self, _slf: &Rc<XdgWmBase>, _serial: u32) {}
}

struct XdgSurfaceHandlerImpl {
    state: Rc<RefCell<SharedState>>,
}

impl XdgSurfaceHandler for XdgSurfaceHandlerImpl {
    fn handle_destroy(&mut self, slf: &Rc<XdgSurface>) {
        let surface_id = slf.unique_id();
        self.state
            .borrow_mut()
            .toplevels
            .retain(|e| e.xdg_surface.unique_id() != surface_id);
        slf.delete_id();
    }

    fn handle_get_toplevel(&mut self, slf: &Rc<XdgSurface>, id: &Rc<XdgToplevel>) {
        id.set_forward_to_server(false);
        id.set_handler(XdgToplevelHandlerImpl {
            state: Rc::clone(&self.state),
        });

        let mut state = self.state.borrow_mut();
        let viewport = state.viewport.unwrap_or_default();
        let serial = state.configure_serial;
        state.configure_serial = serial.wrapping_add(1);

        id.send_configure_bounds(0, 0);
        id.send_configure(viewport.width, viewport.height, &[]);
        slf.send_configure(serial);

        state.toplevels.push(ToplevelEntry {
            xdg_surface: Rc::clone(slf),
            toplevel: Rc::clone(id),
        });
    }

    fn handle_ack_configure(&mut self, _slf: &Rc<XdgSurface>, _serial: u32) {}
}

struct XdgToplevelHandlerImpl {
    state: Rc<RefCell<SharedState>>,
}

impl XdgToplevelHandler for XdgToplevelHandlerImpl {
    fn handle_destroy(&mut self, slf: &Rc<XdgToplevel>) {
        let toplevel_id = slf.unique_id();
        self.state
            .borrow_mut()
            .toplevels
            .retain(|e| e.toplevel.unique_id() != toplevel_id);
        slf.delete_id();
    }
}
