use std::{
    cell::RefCell,
    collections::hash_map::Entry,
    rc::Rc,
};

use wl_proxy::{
    fixed::Fixed,
    object::{
        Object,
        ObjectCoreApi,
    },
    protocols::{
        viewporter::{
            wp_viewport::{
                WpViewport,
                WpViewportError,
                WpViewportHandler,
            },
            wp_viewporter::{
                WpViewporter,
                WpViewporterError,
                WpViewporterHandler,
            },
        },
        wayland::wl_surface::WlSurface,
    },
};

use super::SharedState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SourceRect {
    pub x: Fixed,
    pub y: Fixed,
    pub width: Fixed,
    pub height: Fixed,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct ViewportState {
    pub source: Option<SourceRect>,
    pub destination: Option<(i32, i32)>,
}

pub struct ViewporterHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl WpViewporterHandler for ViewporterHandler {
    fn handle_destroy(&mut self, slf: &Rc<WpViewporter>) {
        slf.delete_id();
    }

    fn handle_get_viewport(
        &mut self, slf: &Rc<WpViewporter>, id: &Rc<WpViewport>, surface: &Rc<WlSurface>,
    ) {
        let surface_id = surface.unique_id();
        let mut state = self.state.borrow_mut();
        if !state.surface_runtime.contains_key(&surface_id) {
            post_viewporter_error(
                slf,
                WpViewporterError::VIEWPORT_EXISTS,
                "viewport target surface does not exist",
            );
            return;
        }
        match state.viewport_states.entry(surface_id) {
            Entry::Occupied(_) => {
                post_viewporter_error(
                    slf,
                    WpViewporterError::VIEWPORT_EXISTS,
                    "surface already has a viewport",
                );
            }
            Entry::Vacant(entry) => {
                entry.insert(ViewportState::default());
                id.set_forward_to_server(false);
                id.set_handler(ViewportHandler {
                    state: Rc::clone(&self.state),
                    surface_id,
                });
            }
        }
    }
}

struct ViewportHandler {
    state: Rc<RefCell<SharedState>>,
    surface_id: u64,
}

impl WpViewportHandler for ViewportHandler {
    fn handle_destroy(&mut self, slf: &Rc<WpViewport>) {
        self.state
            .borrow_mut()
            .viewport_states
            .remove(&self.surface_id);
        slf.delete_id();
    }

    fn handle_set_source(
        &mut self, slf: &Rc<WpViewport>, x: Fixed, y: Fixed, width: Fixed, height: Fixed,
    ) {
        let minus_one = Fixed::from_i32_saturating(-1);
        let unset = x == minus_one && y == minus_one && width == minus_one && height == minus_one;
        let source = if unset {
            None
        } else if x < Fixed::ZERO
            || y < Fixed::ZERO
            || width <= Fixed::ZERO
            || height <= Fixed::ZERO
        {
            post_viewport_error(slf, WpViewportError::BAD_VALUE, "invalid viewport source");
            return;
        } else {
            Some(SourceRect {
                x,
                y,
                width,
                height,
            })
        };

        let mut state = self.state.borrow_mut();
        let Some(viewport) = state.viewport_states.get_mut(&self.surface_id) else {
            post_viewport_error(
                slf,
                WpViewportError::NO_SURFACE,
                "viewport surface no longer exists",
            );
            return;
        };
        viewport.source = source;
    }

    fn handle_set_destination(&mut self, slf: &Rc<WpViewport>, width: i32, height: i32) {
        let destination = if width == -1 && height == -1 {
            None
        } else if width <= 0 || height <= 0 {
            post_viewport_error(
                slf,
                WpViewportError::BAD_VALUE,
                "invalid viewport destination",
            );
            return;
        } else {
            Some((width, height))
        };

        let mut state = self.state.borrow_mut();
        let Some(viewport) = state.viewport_states.get_mut(&self.surface_id) else {
            post_viewport_error(
                slf,
                WpViewportError::NO_SURFACE,
                "viewport surface no longer exists",
            );
            return;
        };
        viewport.destination = destination;
    }
}

fn post_viewporter_error(slf: &Rc<WpViewporter>, error: WpViewporterError, message: &str) {
    if let Some(client) = slf.client() {
        let object: Rc<dyn Object> = slf.clone();
        client.display().send_error(object, error.0, message);
    }
}

fn post_viewport_error(slf: &Rc<WpViewport>, error: WpViewportError, message: &str) {
    if let Some(client) = slf.client() {
        let object: Rc<dyn Object> = slf.clone();
        client.display().send_error(object, error.0, message);
    }
}
