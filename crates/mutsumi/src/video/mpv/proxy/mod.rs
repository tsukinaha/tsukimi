mod dmabuf;
mod drm;
#[cfg(feature = "profiling")]
pub mod profiling;
mod shm;
mod surface;
mod xdg;

#[cfg(feature = "profiling")]
pub use profiling::{
    ProxyProfilingGuard,
    start_proxy_profiling,
};
pub use shm::{
    ShmFrame,
    ShmMemoryFormat,
};

use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{
        IntoRawFd,
        OwnedFd,
    },
    rc::Rc,
    sync::Mutex,
};

use once_cell::sync::Lazy;
use wl_proxy::{
    baseline::Baseline,
    client::ClientHandler,
    global_mapper::GlobalMapper,
    object::{
        Object,
        ObjectCoreApi,
        ObjectRcUtils,
    },
    protocols::{
        ObjectInterface,
        drm::wl_drm::WlDrm,
        fractional_scale_v1::{
            wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
            wp_fractional_scale_v1::WpFractionalScaleV1,
        },
        linux_dmabuf_v1::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        viewporter::wp_viewporter::WpViewporter,
        wayland::{
            wl_callback::WlCallback,
            wl_compositor::WlCompositor,
            wl_display::{
                WlDisplay,
                WlDisplayHandler,
            },
            wl_registry::{
                WlRegistry,
                WlRegistryHandler,
            },
            wl_shm::{
                WlShm,
                WlShmFormat,
            },
            wl_subcompositor::WlSubcompositor,
            wl_surface::WlSurface,
        },
        xdg_shell::xdg_wm_base::XdgWmBase,
    },
    state::{
        Destructor,
        State,
    },
};

use self::{
    dmabuf::{
        ALLOWED_FORMAT_PAIRS,
        BufferInfo,
        DmabufHandler,
    },
    drm::DrmHandler,
    shm::ShmHandler,
    surface::{
        CompositorHandler,
        FractionalScaleManagerHandler,
        SubcompositorHandler,
        ViewporterHandler,
    },
    xdg::{
        ToplevelEntry,
        WmBaseHandler,
    },
};

enum ProxyEvent {
    ReleaseBuffer(u64),
    FrameDone {
        callback_batch_id: u64,
        time_ms: u32,
    },
}

fn send_proxy_event(tx: &flume::Sender<ProxyEvent>, event: ProxyEvent) {
    let _ = tx.send(event);
}

pub struct FrameCallbacks {
    callback_batch_id: u64,
    event_tx: flume::Sender<ProxyEvent>,
}

impl FrameCallbacks {
    pub fn done(self, time_ms: u32) {
        send_proxy_event(
            &self.event_tx,
            ProxyEvent::FrameDone {
                callback_batch_id: self.callback_batch_id,
                time_ms,
            },
        );
    }
}

pub struct DmabufPlane {
    pub fd: OwnedFd,
    pub offset: u32,
    pub stride: u32,
}

pub struct DmabufFrame {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub modifier: u64,
    pub planes: Vec<DmabufPlane>,
    buffer_id: u64,
    event_tx: flume::Sender<ProxyEvent>,
    #[cfg(feature = "profiling")]
    pub profile_frame_id: Option<u64>,
}

impl Drop for DmabufFrame {
    fn drop(&mut self) {
        send_proxy_event(&self.event_tx, ProxyEvent::ReleaseBuffer(self.buffer_id));
    }
}

pub enum SurfaceContentUpdate {
    Unchanged,
    Frame(DmabufFrame),
    Shm(shm::ShmFrame),
    Clear,
}

pub struct SurfaceUpdate {
    pub content: SurfaceContentUpdate,
    pub frame_callbacks: Option<FrameCallbacks>,
}

pub static FRAME_CHANNEL: Lazy<DmabufFrameChannel> = Lazy::new(|| {
    let (tx, rx) = flume::unbounded::<SurfaceUpdate>();
    DmabufFrameChannel { tx, rx }
});

pub struct DmabufFrameChannel {
    pub tx: flume::Sender<SurfaceUpdate>,
    pub rx: flume::Receiver<SurfaceUpdate>,
}

pub static VIEWPORT_CHANNEL: Lazy<ViewportChannel> = Lazy::new(|| {
    let (tx, rx) = flume::unbounded::<(i32, i32, f64)>();
    ViewportChannel { tx, rx }
});

pub struct ViewportChannel {
    pub tx: flume::Sender<(i32, i32, f64)>,
    pub rx: flume::Receiver<(i32, i32, f64)>,
}

static CURRENT_SCALE: Mutex<f64> = Mutex::new(1.0);

struct SharedState {
    buffer_info: HashMap<u64, BufferInfo>,
    event_tx: flume::Sender<ProxyEvent>,
    frame_callbacks: HashMap<u64, Vec<Rc<WlCallback>>>,
    next_callback_batch_id: u64,
    toplevels: Vec<ToplevelEntry>,
    configure_serial: u32,
    fractional_scales: Vec<Rc<WpFractionalScaleV1>>,
    surfaces: Vec<Rc<WlSurface>>,
    shm_buffer_info: HashMap<u64, shm::BufferInfo>,
}

impl SharedState {
    fn configure_toplevels(&mut self, width: i32, height: i32) {
        for entry in &self.toplevels {
            entry.toplevel.send_configure(width, height, &[]);
            entry.xdg_surface.send_configure(self.configure_serial);
            self.configure_serial = self.configure_serial.wrapping_add(1);
        }
    }

    fn update_fractional_scales(&mut self, scale_120: u32) {
        for scale in &self.fractional_scales {
            scale.send_preferred_scale(scale_120);
        }

        let buffer_scale = (scale_120 as f64 / 120.0).ceil() as i32;
        for surface in &self.surfaces {
            if surface.version() >= 6 {
                surface.send_preferred_buffer_scale(buffer_scale);
            }
        }
    }
}

struct DisplayHandler {
    state: Rc<RefCell<SharedState>>,
}

impl WlDisplayHandler for DisplayHandler {
    fn handle_get_registry(&mut self, slf: &Rc<WlDisplay>, registry: &Rc<WlRegistry>) {
        slf.send_get_registry(registry);

        let mut mapper = GlobalMapper::default();
        let xdg_wm_base_client_name =
            mapper.add_synthetic_global(registry, ObjectInterface::XdgWmBase, 4);
        let viewporter_client_name =
            mapper.add_synthetic_global(registry, ObjectInterface::WpViewporter, 1);
        let fractional_scale_manager_client_name =
            mapper.add_synthetic_global(registry, ObjectInterface::WpFractionalScaleManagerV1, 1);
        let shm_client_name = mapper.add_synthetic_global(registry, ObjectInterface::WlShm, 1);

        registry.set_handler(RegistryHandler {
            mapper,
            state: Rc::clone(&self.state),
            xdg_wm_base_client_name,
            viewporter_client_name,
            fractional_scale_manager_client_name,
            shm_client_name,
        });
    }
}

struct RegistryHandler {
    mapper: GlobalMapper,
    state: Rc<RefCell<SharedState>>,
    xdg_wm_base_client_name: u32,
    viewporter_client_name: u32,
    fractional_scale_manager_client_name: u32,
    shm_client_name: u32,
}

impl WlRegistryHandler for RegistryHandler {
    fn handle_global(
        &mut self, slf: &Rc<WlRegistry>, name: u32, interface: ObjectInterface, version: u32,
    ) {
        if interface == ObjectInterface::XdgWmBase
            || interface == ObjectInterface::WpViewporter
            || interface == ObjectInterface::WpFractionalScaleManagerV1
            || interface == ObjectInterface::WlShm
        {
            self.mapper.ignore_global(name);
        } else if interface == ObjectInterface::ZwpLinuxDmabufV1 {
            self.mapper
                .forward_global(slf, name, interface, version.min(4));
        } else {
            self.mapper.forward_global(slf, name, interface, version);
        }
    }

    fn handle_global_remove(&mut self, slf: &Rc<WlRegistry>, name: u32) {
        self.mapper.forward_global_remove(slf, name);
    }

    fn handle_bind(&mut self, slf: &Rc<WlRegistry>, name: u32, id: Rc<dyn Object>) {
        if name == self.xdg_wm_base_client_name {
            let wm_base = id.downcast::<XdgWmBase>();
            wm_base.set_forward_to_server(false);
            wm_base.set_handler(WmBaseHandler {
                state: Rc::clone(&self.state),
            });
        } else if name == self.viewporter_client_name {
            let viewporter = id.downcast::<WpViewporter>();
            viewporter.set_forward_to_server(false);
            viewporter.set_handler(ViewporterHandler);
        } else if name == self.fractional_scale_manager_client_name {
            let manager = id.downcast::<WpFractionalScaleManagerV1>();
            manager.set_forward_to_server(false);
            manager.set_handler(FractionalScaleManagerHandler {
                state: Rc::clone(&self.state),
            });
        } else if name == self.shm_client_name {
            let shm = id.downcast::<WlShm>();
            shm.set_forward_to_server(false);
            shm.set_handler(ShmHandler {
                state: Rc::clone(&self.state),
            });
            shm.send_format(WlShmFormat::ARGB8888);
            shm.send_format(WlShmFormat::XRGB8888);
        } else {
            let compositor = id.try_downcast::<WlCompositor>();
            let subcompositor = id.try_downcast::<WlSubcompositor>();
            let dmabuf = id.try_downcast::<ZwpLinuxDmabufV1>();
            let drm = id.try_downcast::<WlDrm>();

            self.mapper.forward_bind(slf, name, &id);

            if let Some(compositor) = compositor {
                compositor.set_handler(CompositorHandler {
                    state: Rc::clone(&self.state),
                });
            } else if let Some(subcompositor) = subcompositor {
                subcompositor.set_handler(SubcompositorHandler);
            } else if let Some(dmabuf) = dmabuf {
                dmabuf.set_handler(DmabufHandler {
                    state: Rc::clone(&self.state),
                });
            } else if let Some(drm) = drm {
                drm.set_handler(DrmHandler {
                    state: Rc::clone(&self.state),
                });
            }
        }
    }
}

struct ClientHandlerImpl {
    _destructor: Destructor,
}

impl ClientHandler for ClientHandlerImpl {
    fn disconnected(self: Box<Self>) {
        tracing::debug!("wl-proxy-mpv: client disconnected");
    }
}

fn handle_proxy_event(shared: &Rc<RefCell<SharedState>>, event: ProxyEvent) {
    match event {
        ProxyEvent::ReleaseBuffer(buffer_id) => {
            let shared = shared.borrow();
            if let Some(info) = shared.buffer_info.get(&buffer_id) {
                info.buffer.send_release();
            }
        }
        ProxyEvent::FrameDone {
            callback_batch_id,
            time_ms,
        } => {
            if let Some(callbacks) = shared
                .borrow_mut()
                .frame_callbacks
                .remove(&callback_batch_id)
            {
                for callback in callbacks {
                    callback.send_done(time_ms);
                    callback.delete_id();
                }
            }
        }
    }
}

fn handle_viewport_update(shared: &Rc<RefCell<SharedState>>, width: i32, height: i32, scale: f64) {
    shared.borrow_mut().configure_toplevels(width, height);
    *CURRENT_SCALE.lock().unwrap() = scale;
    let scale_120 = (scale * 120.0).round() as u32;
    shared.borrow_mut().update_fractional_scales(scale_120);
}

async fn run_client(
    state: Rc<State>, shared: Rc<RefCell<SharedState>>, event_rx: flume::Receiver<ProxyEvent>,
) {
    let poll_fd = match tokio::io::unix::AsyncFd::new(Rc::clone(state.poll_fd())) {
        Ok(fd) => fd,
        Err(e) => {
            tracing::error!("wl-proxy-mpv: failed to register poll fd: {e}");
            return;
        }
    };

    while state.is_not_destroyed() {
        if let Err(e) = state.dispatch_available() {
            tracing::error!("wl-proxy-mpv: dispatch failed: {e}");
            return;
        }

        if let Err(e) = state.before_poll() {
            tracing::error!("wl-proxy-mpv: failed to prepare poll: {e}");
            return;
        }

        tokio::select! {
            result = poll_fd.readable() => match result {
                Ok(mut guard) => guard.clear_ready(),
                Err(e) => {
                    tracing::error!("wl-proxy-mpv: failed to poll Wayland fd: {e}");
                    return;
                }
            },
            event = event_rx.recv_async() => match event {
                Ok(event) => handle_proxy_event(&shared, event),
                Err(_) => return,
            },
            viewport = VIEWPORT_CHANNEL.rx.recv_async() => match viewport {
                Ok(mut viewport) => {
                    while let Ok(latest) = VIEWPORT_CHANNEL.rx.try_recv() {
                        viewport = latest;
                    }
                    handle_viewport_update(&shared, viewport.0, viewport.1, viewport.2);
                }
                Err(_) => return,
            },
        }
    }
}

fn serve_client(socket: OwnedFd, upstream: String) {
    let state = match State::builder(Baseline::ALL_OF_THEM)
        .with_server_display_name(&upstream)
        .build()
    {
        Ok(state) => state,
        Err(e) => {
            tracing::error!("wl-proxy-mpv: failed to create state: {e}");
            return;
        }
    };
    let client = match state.add_client(&Rc::new(socket)) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("wl-proxy-mpv: failed to add client: {e}");
            return;
        }
    };
    client.set_handler(ClientHandlerImpl {
        _destructor: state.create_destructor(),
    });

    let (event_tx, event_rx) = flume::unbounded();
    let shared = Rc::new(RefCell::new(SharedState {
        buffer_info: HashMap::new(),
        event_tx,
        frame_callbacks: HashMap::new(),
        next_callback_batch_id: 1,
        toplevels: Vec::new(),
        configure_serial: 1,
        fractional_scales: Vec::new(),
        surfaces: Vec::new(),
        shm_buffer_info: HashMap::new(),
    }));
    client.display().set_handler(DisplayHandler {
        state: Rc::clone(&shared),
    });

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!("wl-proxy-mpv: failed to create runtime: {e}");
            return;
        }
    };
    runtime.block_on(run_client(state, shared, event_rx));
}

static PROXY_ARMED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn create_mpv_proxy(format_pairs: Vec<(u32, u64)>) {
    ALLOWED_FORMAT_PAIRS
        .set(format_pairs.into_iter().collect())
        .ok();
}

pub fn arm_mpv_proxy() {
    use std::sync::atomic::Ordering;

    if PROXY_ARMED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let upstream = match std::env::var("WAYLAND_DISPLAY") {
        Ok(upstream) => upstream,
        Err(_) => {
            PROXY_ARMED.store(false, Ordering::SeqCst);
            return;
        }
    };

    let Ok((client, server)) = std::os::unix::net::UnixStream::pair() else {
        PROXY_ARMED.store(false, Ordering::SeqCst);
        return;
    };

    let result = std::thread::Builder::new()
        .name("wl-proxy-mpv".into())
        .spawn(move || {
            serve_client(server.into(), upstream);
            PROXY_ARMED.store(false, Ordering::SeqCst);
        });

    if result.is_err() {
        PROXY_ARMED.store(false, Ordering::SeqCst);
        return;
    }

    unsafe { std::env::set_var("WAYLAND_SOCKET", client.into_raw_fd().to_string()) };
}
