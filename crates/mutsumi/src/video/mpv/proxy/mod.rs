mod buffer_lifecycle;
mod dmabuf;
mod drm;

#[cfg(feature = "profiling")]
pub mod profiling;
mod selection;
mod shm;
mod surface;
mod surface_tree;
mod viewporter;
mod xdg;

#[cfg(feature = "profiling")]
pub use profiling::{
    ProxyProfilingGuard,
    start_proxy_profiling,
};

use std::{
    cell::RefCell,
    collections::HashMap,
    io,
    os::fd::OwnedFd,
    rc::Rc,
    sync::{
        Arc,
        Mutex,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
    thread::JoinHandle,
    time::Duration,
};

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
        linux_dmabuf_v1::zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        viewporter::wp_viewporter::WpViewporter,
        wayland::{
            wl_callback::WlCallback,
            wl_compositor::WlCompositor,
            wl_display::{
                WlDisplay,
                WlDisplayHandler,
            },
            wl_output::{
                WlOutput,
                WlOutputHandler,
                WlOutputMode,
                WlOutputSubpixel,
                WlOutputTransform,
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
        },
        xdg_shell::xdg_wm_base::XdgWmBase,
    },
    state::{
        Destructor,
        State,
    },
};

use self::{
    buffer_lifecycle::{
        BufferAction,
        BufferLifecycle,
    },
    dmabuf::{
        AllowedFormatPairs,
        BufferInfo,
        DmabufHandler,
    },
    drm::DrmHandler,
    shm::ShmHandler,
    surface::{
        CompositorHandler,
        SubcompositorHandler,
        SurfaceRuntime,
    },
    surface_tree::SurfaceTree,
    viewporter::{
        ViewportState,
        ViewporterHandler,
    },
    xdg::{
        ToplevelEntry,
        WmBaseHandler,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferTransport {
    Dmabuf,
    Shm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Viewport {
    width: i32,
    height: i32,
    scale: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            scale: 1.0,
        }
    }
}

#[must_use = "keep the proxy alive for the lifetime of its sink"]
pub struct MpvProxy {
    frame_rx: flume::Receiver<SurfaceUpdate>,
    viewport_tx: flume::Sender<Viewport>,
    viewport: Arc<Mutex<Viewport>>,
    config: ProxyConfig,
}

pub fn create_mpv_proxy(format_pairs: Vec<(u32, u64)>) -> MpvProxy {
    create_mpv_proxy_with_upstream(format_pairs, None)
}

pub fn create_mpv_proxy_with_upstream(
    format_pairs: Vec<(u32, u64)>, upstream_display: Option<String>,
) -> MpvProxy {
    let (frame_tx, frame_rx) = flume::unbounded();
    let (viewport_tx, viewport_rx) = flume::unbounded();
    let viewport = Arc::new(Mutex::new(Viewport::default()));
    let config = ProxyConfig {
        allowed_format_pairs: Arc::new(AllowedFormatPairs::new(format_pairs)),
        frame_tx,
        viewport_rx,
        viewport: Arc::clone(&viewport),
        load_id: Arc::new(AtomicU64::new(0)),
        upstream_display: upstream_display.map(Arc::from),
    };

    MpvProxy {
        frame_rx,
        viewport_tx,
        viewport,
        config,
    }
}

impl MpvProxy {
    pub fn frame_receiver(&self) -> flume::Receiver<SurfaceUpdate> {
        self.frame_rx.clone()
    }

    pub fn update_viewport(&self, width: i32, height: i32, scale: f64) {
        let viewport = Viewport {
            width,
            height,
            scale,
        };
        *self.viewport.lock().unwrap() = viewport;
        let _ = self.viewport_tx.send(viewport);
    }

    pub(crate) fn config(&self) -> ProxyConfig {
        self.config.clone()
    }
}

#[derive(Clone)]
pub(crate) struct ProxyConfig {
    allowed_format_pairs: Arc<AllowedFormatPairs>,
    frame_tx: flume::Sender<SurfaceUpdate>,
    viewport_rx: flume::Receiver<Viewport>,
    viewport: Arc<Mutex<Viewport>>,
    load_id: Arc<AtomicU64>,
    upstream_display: Option<Arc<str>>,
}

impl ProxyConfig {
    pub(crate) fn supports_dmabuf(&self) -> bool {
        !self.allowed_format_pairs.is_empty()
    }

    pub(crate) fn set_load_id(&self, load_id: u64) {
        self.load_id.store(load_id, Ordering::Release);
    }

    pub(crate) fn start(
        &self, generation: u64, transport: BufferTransport,
    ) -> io::Result<ProxyConnection> {
        let (stop_tx, stop_rx) = flume::bounded(1);
        let (connected_tx, connected_rx) = flume::bounded(1);
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let session = SessionConfig {
            generation,
            transport,
            allowed_format_pairs: Arc::clone(&self.allowed_format_pairs),
            frame_tx: self.frame_tx.clone(),
            viewport_rx: self.viewport_rx.clone(),
            initial_viewport: *self.viewport.lock().unwrap(),
            connected_tx,
            load_id: Arc::clone(&self.load_id),
            upstream_display: self.upstream_display.clone(),
        };
        let join = std::thread::Builder::new()
            .name(format!("wl-proxy-mpv-{generation}"))
            .spawn(move || serve_client(session, stop_rx, ready_tx))?;

        match ready_rx.recv() {
            Ok(Ok(client_fd)) => Ok(ProxyConnection {
                client_fd: Some(client_fd),
                connected_rx,
                stop_tx: Some(stop_tx),
                join: Some(join),
            }),
            Ok(Err(error)) => {
                let _ = stop_tx.try_send(());
                let _ = join.join();
                Err(error)
            }
            Err(_) => {
                let _ = join.join();
                Err(io::Error::other(
                    "Wayland proxy thread exited before initialization completed",
                ))
            }
        }
    }
}

#[must_use = "keep the connection alive while the MPV actor owns its client fd"]
pub(crate) struct ProxyConnection {
    client_fd: Option<OwnedFd>,
    connected_rx: flume::Receiver<()>,
    stop_tx: Option<flume::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl ProxyConnection {
    pub(crate) fn take_client_fd(&mut self) -> Option<OwnedFd> {
        self.client_fd.take()
    }

    pub(crate) fn wait_until_connected(&self, timeout: Duration) -> io::Result<()> {
        self.connected_rx.recv_timeout(timeout).map_err(|error| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                format!("MPV did not connect to the synthetic Wayland compositor: {error}"),
            )
        })
    }

    pub(crate) fn stop(&mut self) {
        self.client_fd.take();
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.try_send(());
        }
        if let Some(join) = self.join.take() {
            let _ = std::thread::Builder::new()
                .name("wl-proxy-mpv-cleanup".into())
                .spawn(move || {
                    if join.join().is_err() {
                        tracing::error!("wl-proxy-mpv: proxy thread panicked while stopping");
                    }
                });
        }
    }
}

impl Drop for ProxyConnection {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone)]
struct SessionConfig {
    generation: u64,
    transport: BufferTransport,
    allowed_format_pairs: Arc<AllowedFormatPairs>,
    frame_tx: flume::Sender<SurfaceUpdate>,
    viewport_rx: flume::Receiver<Viewport>,
    initial_viewport: Viewport,
    connected_tx: flume::Sender<()>,
    load_id: Arc<AtomicU64>,
    upstream_display: Option<Arc<str>>,
}

enum ProxyEvent {
    BufferUseEnded(u64),
    FrameDone {
        callback_batch_id: u64,
        time_ms: u32,
    },
}

fn send_proxy_event(tx: &flume::Sender<ProxyEvent>, event: ProxyEvent) {
    let _ = tx.send(event);
}

#[must_use = "frame callbacks must be completed or dropped"]
pub struct FrameCallbacks {
    callback_batch_id: Option<u64>,
    event_tx: flume::Sender<ProxyEvent>,
}

impl FrameCallbacks {
    pub fn done(mut self, time_ms: u32) {
        self.finish(time_ms);
    }

    fn finish(&mut self, time_ms: u32) {
        if let Some(callback_batch_id) = self.callback_batch_id.take() {
            send_proxy_event(
                &self.event_tx,
                ProxyEvent::FrameDone {
                    callback_batch_id,
                    time_ms,
                },
            );
        }
    }
}

impl Drop for FrameCallbacks {
    fn drop(&mut self) {
        self.finish(0);
    }
}

pub(super) struct BufferUseToken {
    buffer_id: u64,
    event_tx: flume::Sender<ProxyEvent>,
}

impl Drop for BufferUseToken {
    fn drop(&mut self) {
        send_proxy_event(&self.event_tx, ProxyEvent::BufferUseEnded(self.buffer_id));
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
    _use_token: BufferUseToken,
    #[cfg(feature = "profiling")]
    pub profile_frame_id: Option<u64>,
}

pub struct ShmFrame {
    pub width: i32,
    pub height: i32,
    pub stride: usize,
    pub format: ShmMemoryFormat,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmMemoryFormat {
    Argb8888,
    Xrgb8888,
}

pub enum CapturedFrame {
    Dmabuf(DmabufFrame),
    Shm(ShmFrame),
}

pub enum SurfaceContentUpdate {
    Unchanged,
    Frame(CapturedFrame),
    Clear,
}

pub struct SurfaceUpdate {
    pub generation: u64,
    pub load_id: u64,
    pub surface_id: u64,
    pub content: SurfaceContentUpdate,
    pub frame_callbacks: Option<FrameCallbacks>,
}

struct SharedState {
    generation: u64,
    load_id: Arc<AtomicU64>,
    allowed_format_pairs: Arc<AllowedFormatPairs>,
    buffer_info: HashMap<u64, BufferInfo>,
    shm_buffer_info: HashMap<u64, shm::BufferInfo>,
    buffer_lifecycle: BufferLifecycle,
    event_tx: flume::Sender<ProxyEvent>,
    frame_tx: flume::Sender<SurfaceUpdate>,
    frame_callbacks: HashMap<u64, Vec<Rc<WlCallback>>>,
    next_callback_batch_id: u64,
    toplevels: Vec<ToplevelEntry>,
    configure_serial: u32,
    viewport: Viewport,
    surface_tree: SurfaceTree,
    surface_runtime: HashMap<u64, SurfaceRuntime>,
    surfaces: HashMap<u64, Rc<wl_proxy::protocols::wayland::wl_surface::WlSurface>>,
    outputs: Vec<Rc<WlOutput>>,
    viewport_states: HashMap<u64, ViewportState>,
}

impl SharedState {
    fn configure_toplevels(&mut self, width: i32, height: i32) {
        for output in &self.outputs {
            send_output_state(output, self.viewport);
        }
        for entry in &self.toplevels {
            entry.toplevel.send_configure(width, height, &entry.states);
            entry.xdg_surface.send_configure(self.configure_serial);
            self.configure_serial = self.configure_serial.wrapping_add(1);
        }
    }

    pub(super) fn register_buffer(&mut self, buffer_id: u64) {
        self.buffer_lifecycle.register(buffer_id);
    }

    pub(super) fn retain_buffer(&mut self, buffer_id: u64) -> bool {
        self.buffer_lifecycle.retain(buffer_id)
    }

    pub(super) fn retire_buffer(&mut self, buffer_id: u64) {
        let action = self.buffer_lifecycle.retire(buffer_id);
        self.apply_buffer_action(action);
    }

    pub(super) fn begin_buffer_use(&mut self, buffer_id: u64) -> Option<BufferUseToken> {
        self.buffer_lifecycle
            .begin_use(buffer_id)
            .then(|| BufferUseToken {
                buffer_id,
                event_tx: self.event_tx.clone(),
            })
    }

    pub(super) fn buffer_destroyed(&mut self, buffer_id: u64) {
        let action = self.buffer_lifecycle.client_destroyed(buffer_id);
        self.apply_buffer_action(action);
    }

    fn end_buffer_use(&mut self, buffer_id: u64) {
        let action = self.buffer_lifecycle.end_use(buffer_id);
        self.apply_buffer_action(action);
    }

    fn apply_buffer_action(&mut self, action: Option<BufferAction>) {
        match action {
            Some(BufferAction::SendRelease(buffer_id)) => {
                if let Some(info) = self.buffer_info.get(&buffer_id) {
                    info.buffer.send_release();
                } else if let Some(info) = self.shm_buffer_info.get(&buffer_id) {
                    info.buffer.send_release();
                }
            }
            Some(BufferAction::Purge(buffer_id)) => {
                self.buffer_info.remove(&buffer_id);
                self.shm_buffer_info.remove(&buffer_id);
            }
            None => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntheticGlobalKind {
    Compositor,
    Subcompositor,
    Shm,
    Output,
    Viewporter,
    XdgWmBase,
    LinuxDmabuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SyntheticGlobal {
    kind: SyntheticGlobalKind,
    version: u32,
}

fn synthetic_global_policy(
    transport: BufferTransport, has_dmabuf_formats: bool,
) -> Vec<SyntheticGlobal> {
    let mut globals = vec![
        SyntheticGlobal {
            kind: SyntheticGlobalKind::Compositor,
            version: 5,
        },
        SyntheticGlobal {
            kind: SyntheticGlobalKind::Subcompositor,
            version: 1,
        },
        SyntheticGlobal {
            kind: SyntheticGlobalKind::Shm,
            version: 1,
        },
        SyntheticGlobal {
            kind: SyntheticGlobalKind::Output,
            version: 2,
        },
        SyntheticGlobal {
            kind: SyntheticGlobalKind::Viewporter,
            version: 1,
        },
        SyntheticGlobal {
            kind: SyntheticGlobalKind::XdgWmBase,
            version: 4,
        },
    ];
    if transport == BufferTransport::Dmabuf && has_dmabuf_formats {
        globals.push(SyntheticGlobal {
            kind: SyntheticGlobalKind::LinuxDmabuf,
            version: 3,
        });
    }
    globals
}

fn send_output_state(output: &WlOutput, viewport: Viewport) {
    let width = viewport.width.max(1);
    let height = viewport.height.max(1);
    let physical_width = (width.saturating_mul(254) / 960).max(1);
    let physical_height = (height.saturating_mul(254) / 960).max(1);
    output.send_geometry(
        0,
        0,
        physical_width,
        physical_height,
        WlOutputSubpixel::UNKNOWN,
        "Mutsumi",
        "Synthetic video output",
        WlOutputTransform::NORMAL,
    );
    output.send_mode(
        WlOutputMode::CURRENT | WlOutputMode::PREFERRED,
        width,
        height,
        60_000,
    );
    output.send_scale(viewport.scale.ceil().max(1.0) as i32);
    output.send_done();
}

struct OutputHandler;

impl WlOutputHandler for OutputHandler {}

struct DisplayHandler {
    state: Rc<RefCell<SharedState>>,
    transport: BufferTransport,
    connected_tx: flume::Sender<()>,
    has_upstream: bool,
}

impl WlDisplayHandler for DisplayHandler {
    fn handle_sync(&mut self, slf: &Rc<WlDisplay>, callback: &Rc<WlCallback>) {
        if self.has_upstream {
            slf.send_sync(callback);
        } else {
            callback.set_forward_to_server(false);
            callback.send_done(0);
            callback.delete_id();
        }
    }

    fn handle_get_registry(&mut self, slf: &Rc<WlDisplay>, registry: &Rc<WlRegistry>) {
        let _ = self.connected_tx.try_send(());
        registry.set_forward_to_server(self.has_upstream);
        if self.has_upstream {
            slf.send_get_registry(registry);
        }
        let mut mapper = GlobalMapper::default();
        let has_dmabuf_formats = !self.state.borrow().allowed_format_pairs.is_empty();
        let mut names = GlobalNames::default();

        for global in
            synthetic_global_policy(self.transport, has_dmabuf_formats && !self.has_upstream)
        {
            let interface = match global.kind {
                SyntheticGlobalKind::Compositor => ObjectInterface::WlCompositor,
                SyntheticGlobalKind::Subcompositor => ObjectInterface::WlSubcompositor,
                SyntheticGlobalKind::Shm => ObjectInterface::WlShm,
                SyntheticGlobalKind::Output => ObjectInterface::WlOutput,
                SyntheticGlobalKind::Viewporter => ObjectInterface::WpViewporter,
                SyntheticGlobalKind::XdgWmBase => ObjectInterface::XdgWmBase,
                SyntheticGlobalKind::LinuxDmabuf => ObjectInterface::ZwpLinuxDmabufV1,
            };
            let name = mapper.add_synthetic_global(registry, interface, global.version);
            match global.kind {
                SyntheticGlobalKind::Compositor => names.compositor = name,
                SyntheticGlobalKind::Subcompositor => names.subcompositor = name,
                SyntheticGlobalKind::Shm => names.shm = name,
                SyntheticGlobalKind::Output => names.output = name,
                SyntheticGlobalKind::Viewporter => names.viewporter = name,
                SyntheticGlobalKind::XdgWmBase => names.xdg_wm_base = name,
                SyntheticGlobalKind::LinuxDmabuf => names.linux_dmabuf = Some(name),
            }
        }

        registry.set_handler(RegistryHandler {
            mapper,
            names,
            state: Rc::clone(&self.state),
            transport: self.transport,
        });
    }
}

#[derive(Default)]
struct GlobalNames {
    compositor: u32,
    subcompositor: u32,
    shm: u32,
    output: u32,
    viewporter: u32,
    xdg_wm_base: u32,
    linux_dmabuf: Option<u32>,
}

struct RegistryHandler {
    mapper: GlobalMapper,
    names: GlobalNames,
    state: Rc<RefCell<SharedState>>,
    transport: BufferTransport,
}

impl WlRegistryHandler for RegistryHandler {
    fn handle_global(
        &mut self, slf: &Rc<WlRegistry>, name: u32, interface: ObjectInterface, version: u32,
    ) {
        if self.transport == BufferTransport::Dmabuf
            && matches!(
                interface,
                ObjectInterface::ZwpLinuxDmabufV1 | ObjectInterface::WlDrm
            )
        {
            let version = if interface == ObjectInterface::ZwpLinuxDmabufV1 {
                version.min(6)
            } else {
                version
            };
            self.mapper.forward_global(slf, name, interface, version);
        } else {
            self.mapper.ignore_global(name);
        }
    }

    fn handle_global_remove(&mut self, slf: &Rc<WlRegistry>, name: u32) {
        self.mapper.forward_global_remove(slf, name);
    }

    fn handle_bind(&mut self, slf: &Rc<WlRegistry>, name: u32, id: Rc<dyn Object>) {
        if name == self.names.compositor {
            let compositor = id.downcast::<WlCompositor>();
            compositor.set_forward_to_server(false);
            compositor.set_handler(CompositorHandler {
                state: Rc::clone(&self.state),
            });
        } else if name == self.names.subcompositor {
            let subcompositor = id.downcast::<WlSubcompositor>();
            subcompositor.set_forward_to_server(false);
            subcompositor.set_handler(SubcompositorHandler {
                state: Rc::clone(&self.state),
            });
        } else if name == self.names.shm {
            let shm = id.downcast::<WlShm>();
            shm.set_forward_to_server(false);
            shm.set_handler(ShmHandler {
                state: Rc::clone(&self.state),
            });
            shm.send_format(WlShmFormat::ARGB8888);
            shm.send_format(WlShmFormat::XRGB8888);
        } else if name == self.names.output {
            let output = id.downcast::<WlOutput>();
            output.set_forward_to_server(false);
            output.set_handler(OutputHandler);
            let mut state = self.state.borrow_mut();
            send_output_state(&output, state.viewport);
            for surface in state.surfaces.values() {
                surface.send_enter(&output);
            }
            state.outputs.push(output);
        } else if name == self.names.viewporter {
            let viewporter = id.downcast::<WpViewporter>();
            viewporter.set_forward_to_server(false);
            viewporter.set_handler(ViewporterHandler {
                state: Rc::clone(&self.state),
            });
        } else if name == self.names.xdg_wm_base {
            let wm_base = id.downcast::<XdgWmBase>();
            wm_base.set_forward_to_server(false);
            wm_base.set_handler(WmBaseHandler {
                state: Rc::clone(&self.state),
            });
        } else if self.names.linux_dmabuf == Some(name) {
            let dmabuf = id.downcast::<ZwpLinuxDmabufV1>();
            dmabuf.set_forward_to_server(false);
            dmabuf.set_handler(DmabufHandler {
                state: Rc::clone(&self.state),
                forward_to_upstream: false,
            });
            dmabuf::advertise_formats(&dmabuf, &self.state.borrow().allowed_format_pairs);
        } else {
            let dmabuf = id.try_downcast::<ZwpLinuxDmabufV1>();
            let drm = id.try_downcast::<WlDrm>();
            if dmabuf.is_some() || drm.is_some() {
                self.mapper.forward_bind(slf, name, &id);
                if let Some(dmabuf) = dmabuf {
                    dmabuf.set_handler(DmabufHandler {
                        state: Rc::clone(&self.state),
                        forward_to_upstream: true,
                    });
                } else if let Some(drm) = drm {
                    drm.set_handler(DrmHandler {
                        state: Rc::clone(&self.state),
                    });
                }
                return;
            }

            id.core().set_forward_to_server(false);
            tracing::warn!(
                name,
                "wl-proxy-mpv: client bound an unknown synthetic global"
            );
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
        ProxyEvent::BufferUseEnded(buffer_id) => {
            shared.borrow_mut().end_buffer_use(buffer_id);
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

fn handle_viewport_update(shared: &Rc<RefCell<SharedState>>, viewport: Viewport) {
    let mut shared = shared.borrow_mut();
    shared.viewport = viewport;
    tracing::trace!(
        width = viewport.width,
        height = viewport.height,
        scale = viewport.scale,
        "wl-proxy-mpv: viewport updated"
    );
    shared.configure_toplevels(viewport.width, viewport.height);
}

async fn run_client(
    state: Rc<State>, shared: Rc<RefCell<SharedState>>, event_rx: flume::Receiver<ProxyEvent>,
    viewport_rx: flume::Receiver<Viewport>, stop_rx: flume::Receiver<()>,
) {
    let poll_fd = match tokio::io::unix::AsyncFd::new(Rc::clone(state.poll_fd())) {
        Ok(fd) => fd,
        Err(error) => {
            tracing::error!("wl-proxy-mpv: failed to register poll fd: {error}");
            return;
        }
    };

    while state.is_not_destroyed() {
        if let Err(error) = state.dispatch_available() {
            if !error.is_destroyed() {
                tracing::error!("wl-proxy-mpv: dispatch failed: {error}");
            }
            return;
        }

        if let Err(error) = state.before_poll() {
            if !error.is_destroyed() {
                tracing::error!("wl-proxy-mpv: failed to prepare poll: {error}");
            }
            return;
        }

        tokio::select! {
            result = poll_fd.readable() => match result {
                Ok(mut guard) => guard.clear_ready(),
                Err(error) => {
                    tracing::error!("wl-proxy-mpv: failed to poll Wayland fd: {error}");
                    return;
                }
            },
            event = event_rx.recv_async() => match event {
                Ok(event) => handle_proxy_event(&shared, event),
                Err(_) => return,
            },
            viewport = viewport_rx.recv_async() => match viewport {
                Ok(mut viewport) => {
                    while let Ok(latest) = viewport_rx.try_recv() {
                        viewport = latest;
                    }
                    handle_viewport_update(&shared, viewport);
                }
                Err(_) => return,
            },
            _ = stop_rx.recv_async() => return,
        }
    }
}

fn serve_client(
    session: SessionConfig, stop_rx: flume::Receiver<()>,
    ready_tx: std::sync::mpsc::SyncSender<io::Result<OwnedFd>>,
) {
    let setup = (|| -> io::Result<_> {
        let upstream_display = (session.transport == BufferTransport::Dmabuf)
            .then_some(session.upstream_display.as_deref())
            .flatten();
        let has_upstream = upstream_display.is_some();
        let builder = State::builder(Baseline::V5);
        let builder = if let Some(upstream) = upstream_display {
            builder.with_server_display_name(upstream)
        } else {
            builder.without_server()
        };
        let state = builder.build().map_err(|error| {
            io::Error::other(format!("failed to create Wayland state: {error}"))
        })?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .map_err(|error| io::Error::other(format!("failed to create runtime: {error}")))?;
        let (client, client_fd) = state
            .connect()
            .map_err(|error| io::Error::other(format!("failed to create client: {error}")))?;
        let server_destructor = state.create_destructor();
        client.set_handler(ClientHandlerImpl {
            _destructor: state.create_destructor(),
        });

        let (event_tx, event_rx) = flume::unbounded();
        let shared = Rc::new(RefCell::new(SharedState {
            generation: session.generation,
            load_id: session.load_id,
            allowed_format_pairs: Arc::clone(&session.allowed_format_pairs),
            buffer_info: HashMap::new(),
            shm_buffer_info: HashMap::new(),
            buffer_lifecycle: BufferLifecycle::default(),
            event_tx,
            frame_tx: session.frame_tx,
            frame_callbacks: HashMap::new(),
            next_callback_batch_id: 1,
            toplevels: Vec::new(),
            configure_serial: 1,
            viewport: session.initial_viewport,
            surface_tree: SurfaceTree::default(),
            surface_runtime: HashMap::new(),
            surfaces: HashMap::new(),
            outputs: Vec::new(),
            viewport_states: HashMap::new(),
        }));
        let display = client.display();
        display.set_forward_to_server(has_upstream);
        display.set_handler(DisplayHandler {
            state: Rc::clone(&shared),
            transport: session.transport,
            connected_tx: session.connected_tx,
            has_upstream,
        });

        Ok((
            state,
            shared,
            event_rx,
            session.viewport_rx,
            runtime,
            server_destructor,
            client_fd,
        ))
    })();

    let (state, shared, event_rx, viewport_rx, runtime, _server_destructor, client_fd) = match setup
    {
        Ok(setup) => setup,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };

    if ready_tx.send(Ok(client_fd)).is_err() {
        return;
    }
    runtime.block_on(run_client(state, shared, event_rx, viewport_rx, stop_rx));
}
