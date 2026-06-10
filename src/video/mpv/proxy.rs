use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    rc::Rc,
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use once_cell::sync::Lazy;

use wl_proxy::protocols::{
    ObjectInterface,
    linux_dmabuf_v1::{
        zwp_linux_buffer_params_v1::{
            ZwpLinuxBufferParamsV1, ZwpLinuxBufferParamsV1Flags, ZwpLinuxBufferParamsV1Handler,
        },
        zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        zwp_linux_dmabuf_v1::{ZwpLinuxDmabufV1, ZwpLinuxDmabufV1Handler},
    },
    viewporter::{
        wp_viewport::{WpViewport, WpViewportHandler},
        wp_viewporter::{WpViewporter, WpViewporterHandler},
    },
    wayland::{
        wl_buffer::{WlBuffer, WlBufferHandler},
        wl_callback::WlCallback,
        wl_compositor::{WlCompositor, WlCompositorHandler},
        wl_display::{WlDisplay, WlDisplayHandler},
        wl_registry::{WlRegistry, WlRegistryHandler},
        wl_subcompositor::{WlSubcompositor, WlSubcompositorHandler},
        wl_subsurface::{WlSubsurface, WlSubsurfaceHandler},
        wl_surface::{WlSurface, WlSurfaceHandler},
    },
    xdg_shell::{
        xdg_surface::{XdgSurface, XdgSurfaceHandler},
        xdg_toplevel::{XdgToplevel, XdgToplevelHandler},
        xdg_wm_base::{XdgWmBase, XdgWmBaseHandler},
    },
};
use wl_proxy::{
    baseline::Baseline,
    client::ClientHandler,
    global_mapper::GlobalMapper,
    object::{Object, ObjectCoreApi, ObjectRcUtils},
    state::{Destructor, State},
};

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
}

pub static FRAME_CHANNEL: Lazy<DmabufFrameChannel> = Lazy::new(|| {
    let (tx, rx) = flume::unbounded::<DmabufFrame>();
    DmabufFrameChannel { tx, rx }
});

pub struct DmabufFrameChannel {
    pub tx: flume::Sender<DmabufFrame>,
    pub rx: flume::Receiver<DmabufFrame>,
}

pub static SIZE_CHANNEL: Lazy<SizeChannel> = Lazy::new(|| {
    let (tx, rx) = flume::unbounded::<(i32, i32)>();
    SizeChannel { tx, rx }
});

pub struct SizeChannel {
    pub tx: flume::Sender<(i32, i32)>,
    pub rx: flume::Receiver<(i32, i32)>,
}

static CURRENT_SIZE: Mutex<(i32, i32)> = Mutex::new((0, 0));

struct StoredPlane {
    fd: OwnedFd,
    offset: u32,
    stride: u32,
}

struct BufferInfo {
    _buffer: Rc<WlBuffer>,
    planes: Vec<StoredPlane>,
    width: u32,
    height: u32,
    format: u32,
    modifier: u64,
}

impl BufferInfo {
    fn to_frame(&self) -> DmabufFrame {
        let planes = self
            .planes
            .iter()
            .map(|p| {
                let raw = unsafe { libc::dup(p.fd.as_raw_fd()) };
                DmabufPlane {
                    fd: unsafe { OwnedFd::from_raw_fd(raw) },
                    offset: p.offset,
                    stride: p.stride,
                }
            })
            .collect();

        DmabufFrame {
            width: self.width,
            height: self.height,
            format: self.format,
            modifier: self.modifier,
            planes,
        }
    }
}

struct ToplevelEntry {
    xdg_surface: Rc<XdgSurface>,
    toplevel: Rc<XdgToplevel>,
}

struct SharedState {
    buffer_info: HashMap<u64, BufferInfo>,
    toplevels: Vec<ToplevelEntry>,
    configure_serial: u32,
}

impl SharedState {
    fn configure_toplevels(&mut self, width: i32, height: i32) {
        for entry in &self.toplevels {
            entry.toplevel.send_configure(width, height, &[]);
            entry.xdg_surface.send_configure(self.configure_serial);
            self.configure_serial = self.configure_serial.wrapping_add(1);
        }
    }
}

fn current_time_ms() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u32)
        .unwrap_or(0)
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

        registry.set_handler(RegistryHandler {
            mapper,
            state: Rc::clone(&self.state),
            xdg_wm_base_client_name,
            viewporter_client_name,
        });
    }
}

struct RegistryHandler {
    mapper: GlobalMapper,
    state: Rc<RefCell<SharedState>>,
    xdg_wm_base_client_name: u32,
    viewporter_client_name: u32,
}

impl WlRegistryHandler for RegistryHandler {
    fn handle_global(
        &mut self,
        slf: &Rc<WlRegistry>,
        name: u32,
        interface: ObjectInterface,
        version: u32,
    ) {
        if interface == ObjectInterface::XdgWmBase || interface == ObjectInterface::WpViewporter {
            self.mapper.ignore_global(name);
        } else if interface == ObjectInterface::ZwpLinuxDmabufV1 {
            self.mapper
                .forward_global(slf, name, interface, version.min(3));
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
        } else {
            let compositor = id.try_downcast::<WlCompositor>();
            let subcompositor = id.try_downcast::<WlSubcompositor>();
            let dmabuf = id.try_downcast::<ZwpLinuxDmabufV1>();

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
            }
        }
    }
}

struct CompositorHandler {
    state: Rc<RefCell<SharedState>>,
}

impl WlCompositorHandler for CompositorHandler {
    fn handle_create_surface(&mut self, _slf: &Rc<WlCompositor>, id: &Rc<WlSurface>) {
        id.set_forward_to_server(false);
        id.set_handler(SurfaceHandler {
            shared: Rc::clone(&self.state),
            pending_buffer: None,
            pending_callbacks: Vec::new(),
        });
    }
}

struct SubcompositorHandler;

impl WlSubcompositorHandler for SubcompositorHandler {
    fn handle_get_subsurface(
        &mut self,
        _slf: &Rc<WlSubcompositor>,
        id: &Rc<WlSubsurface>,
        _surface: &Rc<WlSurface>,
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

struct ViewporterHandler;

impl WpViewporterHandler for ViewporterHandler {
    fn handle_destroy(&mut self, slf: &Rc<WpViewporter>) {
        slf.delete_id();
    }

    fn handle_get_viewport(
        &mut self,
        _slf: &Rc<WpViewporter>,
        id: &Rc<WpViewport>,
        _surface: &Rc<WlSurface>,
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

struct SurfaceHandler {
    shared: Rc<RefCell<SharedState>>,
    pending_buffer: Option<Rc<WlBuffer>>,
    pending_callbacks: Vec<Rc<WlCallback>>,
}

impl WlSurfaceHandler for SurfaceHandler {
    fn handle_destroy(&mut self, slf: &Rc<WlSurface>) {
        slf.delete_id();
    }

    fn handle_attach(
        &mut self,
        _slf: &Rc<WlSurface>,
        buffer: Option<&Rc<WlBuffer>>,
        _x: i32,
        _y: i32,
    ) {
        self.pending_buffer = buffer.map(Rc::clone);
    }

    fn handle_frame(&mut self, _slf: &Rc<WlSurface>, callback: &Rc<WlCallback>) {
        self.pending_callbacks.push(Rc::clone(callback));
    }

    fn handle_commit(&mut self, _slf: &Rc<WlSurface>) {
        let time_ms = current_time_ms();

        for cb in std::mem::take(&mut self.pending_callbacks) {
            cb.send_done(time_ms);
            cb.delete_id();
        }

        if let Some(buffer) = self.pending_buffer.take() {
            let state = self.shared.borrow();
            if let Some(info) = state.buffer_info.get(&buffer.unique_id()) {
                let frame = info.to_frame();
                let _ = FRAME_CHANNEL.tx.send(frame);
            }
            buffer.send_release();
        }
    }
}

const fn drm_fourcc(code: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*code)
}

const ALLOWED_FORMATS: [u32; 4] = [
    drm_fourcc(b"AR24"), // ARGB8888
    drm_fourcc(b"XR24"), // XRGB8888
    drm_fourcc(b"AB24"), // ABGR8888
    drm_fourcc(b"XB24"), // XBGR8888
];

struct DmabufHandler {
    state: Rc<RefCell<SharedState>>,
}

impl ZwpLinuxDmabufV1Handler for DmabufHandler {
    fn handle_format(&mut self, slf: &Rc<ZwpLinuxDmabufV1>, format: u32) {
        if ALLOWED_FORMATS.contains(&format) {
            slf.send_format(format);
        }
    }

    fn handle_modifier(
        &mut self,
        slf: &Rc<ZwpLinuxDmabufV1>,
        format: u32,
        modifier_hi: u32,
        modifier_lo: u32,
    ) {
        if ALLOWED_FORMATS.contains(&format) {
            slf.send_modifier(format, modifier_hi, modifier_lo);
        }
    }

    fn handle_create_params(
        &mut self,
        _slf: &Rc<ZwpLinuxDmabufV1>,
        params_id: &Rc<ZwpLinuxBufferParamsV1>,
    ) {
        params_id.set_forward_to_server(false);
        params_id.set_handler(BufferParamsHandler {
            state: Rc::clone(&self.state),
            planes: Vec::new(),
            modifier: 0,
        });
    }

    fn handle_get_surface_feedback(
        &mut self,
        slf: &Rc<ZwpLinuxDmabufV1>,
        id: &Rc<ZwpLinuxDmabufFeedbackV1>,
        _surface: &Rc<WlSurface>,
    ) {
        slf.send_get_default_feedback(id);
    }
}

struct BufferParamsHandler {
    state: Rc<RefCell<SharedState>>,
    planes: Vec<StoredPlane>,
    modifier: u64,
}

impl ZwpLinuxBufferParamsV1Handler for BufferParamsHandler {
    fn handle_destroy(&mut self, slf: &Rc<ZwpLinuxBufferParamsV1>) {
        slf.delete_id();
    }

    fn handle_add(
        &mut self,
        _slf: &Rc<ZwpLinuxBufferParamsV1>,
        fd: &Rc<OwnedFd>,
        _plane_idx: u32,
        offset: u32,
        stride: u32,
        modifier_hi: u32,
        modifier_lo: u32,
    ) {
        let raw = unsafe { libc::dup(fd.as_raw_fd()) };
        let dup_fd = unsafe { OwnedFd::from_raw_fd(raw) };

        self.modifier = ((modifier_hi as u64) << 32) | (modifier_lo as u64);
        self.planes.push(StoredPlane {
            fd: dup_fd,
            offset,
            stride,
        });
    }

    fn handle_create_immed(
        &mut self,
        _slf: &Rc<ZwpLinuxBufferParamsV1>,
        buffer_id: &Rc<WlBuffer>,
        width: i32,
        height: i32,
        format: u32,
        _flags: ZwpLinuxBufferParamsV1Flags,
    ) {
        buffer_id.set_forward_to_server(false);
        buffer_id.set_handler(WlBufferHandlerImpl {
            shared: Rc::clone(&self.state),
        });

        let info = BufferInfo {
            _buffer: Rc::clone(buffer_id),
            planes: std::mem::take(&mut self.planes),
            width: width as u32,
            height: height as u32,
            format,
            modifier: self.modifier,
        };

        self.state
            .borrow_mut()
            .buffer_info
            .insert(buffer_id.unique_id(), info);
    }
}

struct WlBufferHandlerImpl {
    shared: Rc<RefCell<SharedState>>,
}

impl WlBufferHandler for WlBufferHandlerImpl {
    fn handle_destroy(&mut self, slf: &Rc<WlBuffer>) {
        self.shared
            .borrow_mut()
            .buffer_info
            .remove(&slf.unique_id());

        slf.delete_id();
    }
}

struct WmBaseHandler {
    state: Rc<RefCell<SharedState>>,
}

impl XdgWmBaseHandler for WmBaseHandler {
    fn handle_destroy(&mut self, slf: &Rc<XdgWmBase>) {
        slf.delete_id();
    }

    fn handle_get_xdg_surface(
        &mut self,
        _slf: &Rc<XdgWmBase>,
        id: &Rc<XdgSurface>,
        _surface: &Rc<WlSurface>,
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

        if id.version() >= XdgToplevel::MSG__CONFIGURE_BOUNDS__SINCE {
            id.send_configure_bounds(1920, 1080);
        }

        let (width, height) = *CURRENT_SIZE.lock().unwrap();
        let mut state = self.state.borrow_mut();
        id.send_configure(width, height, &[]);
        let serial = state.configure_serial;
        state.configure_serial = serial.wrapping_add(1);
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

struct ClientHandlerImpl {
    _destructor: Destructor,
}

impl ClientHandler for ClientHandlerImpl {
    fn disconnected(self: Box<Self>) {
        tracing::debug!("wl-proxy-mpv: client disconnected");
    }
}

fn serve_client(socket: OwnedFd, upstream: String) {
    // Pin the upstream explicitly: the builder would otherwise consume the
    // WAYLAND_SOCKET env var that is reserved for mpv's own connection.
    let state = match State::builder(Baseline::ALL_OF_THEM)
        .with_server_display_name(&upstream)
        .build()
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("wl-proxy-mpv: failed to create state: {e}");
            return;
        }
    };
    let client = match state.add_client(&Rc::new(socket)) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("wl-proxy-mpv: failed to add client: {e}");
            return;
        }
    };
    client.set_handler(ClientHandlerImpl {
        _destructor: state.create_destructor(),
    });

    let shared = Rc::new(RefCell::new(SharedState {
        buffer_info: HashMap::new(),
        toplevels: Vec::new(),
        configure_serial: 1,
    }));
    client.display().set_handler(DisplayHandler {
        state: Rc::clone(&shared),
    });

    while state.is_not_destroyed() {
        if let Err(e) = state.dispatch(Some(Duration::from_millis(50))) {
            tracing::error!("wl-proxy-mpv: dispatch failed: {e}");
            return;
        }

        let mut latest = None;
        while let Ok(size) = SIZE_CHANNEL.rx.try_recv() {
            latest = Some(size);
        }
        if let Some((width, height)) = latest {
            *CURRENT_SIZE.lock().unwrap() = (width, height);
            shared.borrow_mut().configure_toplevels(width, height);
        }
    }
}

/// Spawns the proxy serving a private socketpair and returns the client end,
/// meant to be handed to libwayland via `WAYLAND_SOCKET`. The proxy's upstream
/// connection keeps following the untouched `WAYLAND_DISPLAY`.
pub fn create_mpv_proxy() -> Option<OwnedFd> {
    let upstream = std::env::var("WAYLAND_DISPLAY").ok()?;
    let (client, server) = std::os::unix::net::UnixStream::pair().ok()?;

    std::thread::Builder::new()
        .name("wl-proxy-mpv".into())
        .spawn(move || serve_client(server.into(), upstream))
        .ok()?;

    Some(client.into())
}
