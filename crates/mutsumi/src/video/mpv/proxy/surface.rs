use std::{
    cell::RefCell,
    rc::Rc,
};

use wl_proxy::{
    object::ObjectCoreApi,
    protocols::{
        fractional_scale_v1::{
            wp_fractional_scale_manager_v1::{
                WpFractionalScaleManagerV1,
                WpFractionalScaleManagerV1Handler,
            },
            wp_fractional_scale_v1::{
                WpFractionalScaleV1,
                WpFractionalScaleV1Handler,
            },
        },
        viewporter::{
            wp_viewport::{
                WpViewport,
                WpViewportHandler,
            },
            wp_viewporter::{
                WpViewporter,
                WpViewporterHandler,
            },
        },
        wayland::{
            wl_buffer::WlBuffer,
            wl_callback::WlCallback,
            wl_compositor::{
                WlCompositor,
                WlCompositorHandler,
            },
            wl_subcompositor::{
                WlSubcompositor,
                WlSubcompositorHandler,
            },
            wl_subsurface::{
                WlSubsurface,
                WlSubsurfaceHandler,
            },
            wl_surface::{
                WlSurface,
                WlSurfaceHandler,
            },
        },
    },
};

use super::{
    CURRENT_SCALE,
    FRAME_CHANNEL,
    FrameCallbacks,
    SharedState,
    SurfaceContentUpdate,
    SurfaceUpdate,
};

pub struct CompositorHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl WlCompositorHandler for CompositorHandler {
    fn handle_create_surface(&mut self, _slf: &Rc<WlCompositor>, id: &Rc<WlSurface>) {
        id.set_forward_to_server(false);
        id.set_handler(SurfaceHandler {
            shared: Rc::clone(&self.state),
            pending_buffer: None,
            pending_callbacks: Vec::new(),
        });

        let mut state = self.state.borrow_mut();
        if id.version() >= 6 {
            let scale = *CURRENT_SCALE.lock().unwrap();
            id.send_preferred_buffer_scale(scale.ceil() as i32);
        }
        state.surfaces.push(Rc::clone(id));
    }
}

pub struct SubcompositorHandler;

impl WlSubcompositorHandler for SubcompositorHandler {
    fn handle_get_subsurface(
        &mut self, _slf: &Rc<WlSubcompositor>, id: &Rc<WlSubsurface>, _surface: &Rc<WlSurface>,
        _parent: &Rc<WlSurface>,
    ) {
        id.set_forward_to_server(false);
        id.set_handler(SubsurfaceHandler);
    }
}

struct SubsurfaceHandler;

impl WlSubsurfaceHandler for SubsurfaceHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlSubsurface>) {
        slf.delete_id();
    }
}

pub struct ViewporterHandler;

impl WpViewporterHandler for ViewporterHandler {
    fn handle_destroy(&mut self, slf: &Rc<WpViewporter>) {
        slf.delete_id();
    }

    fn handle_get_viewport(
        &mut self, _slf: &Rc<WpViewporter>, id: &Rc<WpViewport>, _surface: &Rc<WlSurface>,
    ) {
        id.set_forward_to_server(false);
        id.set_handler(ViewportHandler);
    }
}

struct ViewportHandler;

impl WpViewportHandler for ViewportHandler {
    fn handle_destroy(&mut self, slf: &Rc<WpViewport>) {
        slf.delete_id();
    }
}

pub struct FractionalScaleManagerHandler {
    pub state: Rc<RefCell<SharedState>>,
}

impl WpFractionalScaleManagerV1Handler for FractionalScaleManagerHandler {
    fn handle_destroy(&mut self, slf: &Rc<WpFractionalScaleManagerV1>) {
        slf.delete_id();
    }

    fn handle_get_fractional_scale(
        &mut self, _slf: &Rc<WpFractionalScaleManagerV1>, id: &Rc<WpFractionalScaleV1>,
        _surface: &Rc<WlSurface>,
    ) {
        id.set_forward_to_server(false);
        let scale_120 = (*CURRENT_SCALE.lock().unwrap() * 120.0).round() as u32;
        id.send_preferred_scale(scale_120);
        id.set_handler(FractionalScaleHandler {
            state: Rc::clone(&self.state),
        });
        self.state
            .borrow_mut()
            .fractional_scales
            .push(Rc::clone(id));
    }
}

struct FractionalScaleHandler {
    state: Rc<RefCell<SharedState>>,
}

impl WpFractionalScaleV1Handler for FractionalScaleHandler {
    fn handle_destroy(&mut self, slf: &Rc<WpFractionalScaleV1>) {
        self.state
            .borrow_mut()
            .fractional_scales
            .retain(|s| !Rc::ptr_eq(s, slf));
        slf.delete_id();
    }
}

struct SurfaceHandler {
    shared: Rc<RefCell<SharedState>>,
    pending_buffer: Option<Option<Rc<WlBuffer>>>,
    pending_callbacks: Vec<Rc<WlCallback>>,
}

impl WlSurfaceHandler for SurfaceHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlSurface>) {
        let surface_id = slf.unique_id();
        self.shared
            .borrow_mut()
            .surfaces
            .retain(|surface| surface.unique_id() != surface_id);
        slf.delete_id();
    }

    fn handle_attach(
        &mut self, _slf: &Rc<WlSurface>, buffer: Option<&Rc<WlBuffer>>, _x: i32, _y: i32,
    ) {
        self.pending_buffer = Some(buffer.map(Rc::clone));
    }

    fn handle_frame(&mut self, _slf: &Rc<WlSurface>, callback: &Rc<WlCallback>) {
        self.pending_callbacks.push(Rc::clone(callback));
    }

    fn handle_commit(&mut self, _slf: &Rc<WlSurface>) {
        let mut state = self.shared.borrow_mut();
        let callbacks = std::mem::take(&mut self.pending_callbacks);
        let frame_callbacks = if callbacks.is_empty() {
            None
        } else {
            let callback_batch_id = state.next_callback_batch_id;
            state.next_callback_batch_id = callback_batch_id.wrapping_add(1);
            state.frame_callbacks.insert(callback_batch_id, callbacks);

            Some(FrameCallbacks {
                callback_batch_id,
                event_tx: state.event_tx.clone(),
            })
        };

        let content = match self.pending_buffer.take() {
            None => SurfaceContentUpdate::Unchanged,
            Some(None) => SurfaceContentUpdate::Clear,
            Some(Some(buffer)) => {
                if let Some(info) = state.buffer_info.get(&buffer.unique_id()) {
                    match info.to_frame(buffer.unique_id(), state.event_tx.clone()) {
                        Some(frame) => SurfaceContentUpdate::Frame(frame),
                        None => {
                            tracing::warn!(
                                buffer_id = buffer.unique_id(),
                                "wl-proxy-mpv: attached buffer was not created through a supported dmabuf protocol"
                            );
                            buffer.send_release();
                            SurfaceContentUpdate::Clear
                        }
                    }
                } else if let Some(info) = state.shm_buffer_info.get(&buffer.unique_id()) {
                    let content = info
                        .snapshot()
                        .map(|snapshot| SurfaceContentUpdate::Shm(snapshot.to_frame()))
                        .unwrap_or(SurfaceContentUpdate::Clear);
                    // wl_shm is copy semantics: the bytes were captured by the
                    // snapshot, so the buffer can be released immediately,
                    // unlike dmabuf which keeps the fds until the frame drops.
                    buffer.send_release();
                    content
                } else {
                    buffer.send_release();
                    SurfaceContentUpdate::Clear
                }
            }
        };
        drop(state);

        if !matches!(content, SurfaceContentUpdate::Unchanged) || frame_callbacks.is_some() {
            let _ = FRAME_CHANNEL.tx.send(SurfaceUpdate {
                content,
                frame_callbacks,
            });
        }
    }
}
