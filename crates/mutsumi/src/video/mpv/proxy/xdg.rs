use std::{
    cell::RefCell,
    rc::Rc,
};

use wl_proxy::{
    object::ObjectCoreApi,
    protocols::{
        wayland::{
            wl_output::WlOutput,
            wl_surface::WlSurface,
        },
        xdg_shell::{
            xdg_surface::{
                XdgSurface,
                XdgSurfaceHandler,
            },
            xdg_toplevel::{
                XdgToplevel,
                XdgToplevelHandler,
                XdgToplevelState,
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
    pub wl_surface_id: u64,
    pub states: Vec<u8>,
}

pub struct WmBaseHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl XdgWmBaseHandler for WmBaseHandler {
    fn handle_destroy(&mut self, slf: &Rc<XdgWmBase>) {
        slf.delete_id();
    }

    fn handle_get_xdg_surface(
        &mut self, _slf: &Rc<XdgWmBase>, id: &Rc<XdgSurface>, surface: &Rc<WlSurface>,
    ) {
        id.set_forward_to_server(false);
        id.set_handler(XdgSurfaceHandlerImpl {
            state: Rc::clone(&self.state),
            wl_surface: Rc::clone(surface),
        });
    }

    fn handle_pong(&mut self, _slf: &Rc<XdgWmBase>, _serial: u32) {}
}

struct XdgSurfaceHandlerImpl {
    state: Rc<RefCell<SharedState>>,
    wl_surface: Rc<WlSurface>,
}

impl XdgSurfaceHandler for XdgSurfaceHandlerImpl {
    fn handle_destroy(&mut self, slf: &Rc<XdgSurface>) {
        let xdg_surface_id = slf.unique_id();
        let wl_surface_id = self.wl_surface.unique_id();
        let mut state = self.state.borrow_mut();
        state
            .toplevels
            .retain(|entry| entry.xdg_surface.unique_id() != xdg_surface_id);
        state.clear_surface_role(wl_surface_id);
        slf.delete_id();
    }

    fn handle_get_toplevel(&mut self, slf: &Rc<XdgSurface>, id: &Rc<XdgToplevel>) {
        id.set_forward_to_server(false);
        let wl_surface_id = self.wl_surface.unique_id();
        id.set_handler(XdgToplevelHandlerImpl {
            state: Rc::clone(&self.state),
            wl_surface_id,
        });

        let mut state = self.state.borrow_mut();
        state.assign_xdg_root(wl_surface_id);
        let viewport = state.viewport;
        id.send_configure(viewport.width, viewport.height, &[]);
        if id.version() >= 4 {
            id.send_configure_bounds(viewport.width.max(0), viewport.height.max(0));
        }
        let serial = state.configure_serial;
        state.configure_serial = serial.wrapping_add(1);
        slf.send_configure(serial);
        state.toplevels.push(ToplevelEntry {
            xdg_surface: Rc::clone(slf),
            toplevel: Rc::clone(id),
            wl_surface_id,
            states: Vec::new(),
        });
    }

    fn handle_ack_configure(&mut self, _slf: &Rc<XdgSurface>, _serial: u32) {}
}

struct XdgToplevelHandlerImpl {
    state: Rc<RefCell<SharedState>>,
    wl_surface_id: u64,
}

impl XdgToplevelHandler for XdgToplevelHandlerImpl {
    fn handle_destroy(&mut self, slf: &Rc<XdgToplevel>) {
        let toplevel_id = slf.unique_id();
        let mut state = self.state.borrow_mut();
        state
            .toplevels
            .retain(|entry| entry.toplevel.unique_id() != toplevel_id);
        if !state
            .toplevels
            .iter()
            .any(|entry| entry.wl_surface_id == self.wl_surface_id)
        {
            state.clear_surface_role(self.wl_surface_id);
        }
        slf.delete_id();
    }

    fn handle_set_maximized(&mut self, slf: &Rc<XdgToplevel>) {
        self.configure_state(slf, Some(XdgToplevelState::MAXIMIZED));
    }

    fn handle_unset_maximized(&mut self, slf: &Rc<XdgToplevel>) {
        self.configure_state(slf, None);
    }

    fn handle_set_fullscreen(&mut self, slf: &Rc<XdgToplevel>, _output: Option<&Rc<WlOutput>>) {
        self.configure_state(slf, Some(XdgToplevelState::FULLSCREEN));
    }

    fn handle_unset_fullscreen(&mut self, slf: &Rc<XdgToplevel>) {
        self.configure_state(slf, None);
    }
}

impl XdgToplevelHandlerImpl {
    fn configure_state(&self, toplevel: &Rc<XdgToplevel>, state: Option<XdgToplevelState>) {
        let mut shared = self.state.borrow_mut();
        let viewport = shared.viewport;
        let states = state.map_or_else(Vec::new, |state| state.0.to_ne_bytes().to_vec());
        let xdg_surface = shared
            .toplevels
            .iter_mut()
            .find(|entry| entry.toplevel.unique_id() == toplevel.unique_id())
            .map(|entry| {
                entry.states.clone_from(&states);
                Rc::clone(&entry.xdg_surface)
            });
        toplevel.send_configure(viewport.width, viewport.height, &states);
        if let Some(xdg_surface) = xdg_surface {
            let serial = shared.configure_serial;
            shared.configure_serial = serial.wrapping_add(1);
            xdg_surface.send_configure(serial);
        }
    }
}
