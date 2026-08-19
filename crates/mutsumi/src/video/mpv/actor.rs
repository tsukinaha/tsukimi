use std::{
    cell::Cell,
    collections::HashMap,
    ops::Deref,
    os::fd::IntoRawFd,
    sync::{
        Arc,
        Mutex,
        OnceLock,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
};

use crate::MutsumiMpvError;

use super::{
    ListenEvent,
    logging,
    proxy::{
        BufferTransport,
        ProxyConfig,
        ProxyConnection,
    },
    session::{
        CandidateFailure,
        MpvSessionEvent,
        MpvSessionOptions,
        RendererCandidate,
    },
};
use flume::{
    Receiver,
    Sender,
    unbounded,
};
use libmpv2::{
    Format,
    Mpv,
    events::{
        Event,
        PropertyData,
    },
};
use serde_json::Value;

static WAYLAND_HANDOFF_LOCK: Mutex<()> = Mutex::new(());

type MpvInitializer =
    dyn Fn(libmpv2::MpvInitializer) -> libmpv2::Result<()> + Send + Sync + 'static;

static MPV_INITIALIZER: OnceLock<Box<MpvInitializer>> = OnceLock::new();
static DEFAULT_MPV_SESSION_OPTIONS: OnceLock<MpvSessionOptions> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the default mpv session options have already been set")]
pub struct DefaultMpvSessionOptionsAlreadySet;

pub fn set_default_mpv_session_options(
    options: MpvSessionOptions,
) -> Result<(), DefaultMpvSessionOptionsAlreadySet> {
    DEFAULT_MPV_SESSION_OPTIONS
        .set(options)
        .map_err(|_| DefaultMpvSessionOptionsAlreadySet)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the mpv initializer has already been set")]
pub struct MpvInitializerAlreadySet;

pub fn set_mpv_initializer<F>(initializer: F) -> Result<(), MpvInitializerAlreadySet>
where
    F: Fn(libmpv2::MpvInitializer) -> libmpv2::Result<()> + Send + Sync + 'static,
{
    MPV_INITIALIZER
        .set(Box::new(initializer))
        .map_err(|_| MpvInitializerAlreadySet)
}

struct SendMpv {
    mpv: Arc<Mpv>,
    has_file: Cell<bool>,
    generation: u64,
    actor_tx: Sender<ActorRequest>,
}

unsafe impl Send for SendMpv {}

#[derive(Debug, Clone)]
pub enum MpvValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub enum MpvValueType {
    Bool,
    I64,
    F64,
    String,
}

impl From<i32> for MpvValue {
    fn from(val: i32) -> Self {
        MpvValue::I64(val as i64)
    }
}

impl From<u32> for MpvValue {
    fn from(val: u32) -> Self {
        MpvValue::I64(val as i64)
    }
}

impl From<bool> for MpvValue {
    fn from(val: bool) -> Self {
        MpvValue::Bool(val)
    }
}

impl From<i64> for MpvValue {
    fn from(val: i64) -> Self {
        MpvValue::I64(val)
    }
}

impl From<f64> for MpvValue {
    fn from(val: f64) -> Self {
        MpvValue::F64(val)
    }
}

impl From<String> for MpvValue {
    fn from(val: String) -> Self {
        MpvValue::String(val)
    }
}

impl From<&str> for MpvValue {
    fn from(val: &str) -> Self {
        MpvValue::String(val.to_string())
    }
}

impl MpvValue {
    pub fn set_on(&self, mpv: &Mpv, property: &str) -> libmpv2::Result<()> {
        match self {
            MpvValue::Bool(value) => mpv.set_property(property, *value),
            MpvValue::I64(value) => mpv.set_property(property, *value),
            MpvValue::F64(value) => mpv.set_property(property, *value),
            MpvValue::String(value) => mpv.set_property(property, value.as_str()),
        }
    }
}

pub enum MpvMessage {
    Command {
        cmd: String,
        args: Vec<String>,
    },
    SetProperty {
        property: String,
        value: MpvValue,
    },
    GetProperty {
        property: String,
        value_type: MpvValueType,
        tx: tokio::sync::oneshot::Sender<MpvValue>,
    },
    InitRenderContext(tokio::sync::oneshot::Sender<Arc<Mpv>>),
    Shutdown,
}

struct EventHub<T> {
    subscribers: Mutex<Vec<Sender<T>>>,
}

impl<T> Default for EventHub<T> {
    fn default() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
        }
    }
}

impl<T: Clone> EventHub<T> {
    fn subscribe(&self) -> Receiver<T> {
        let (tx, rx) = unbounded();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    fn emit(&self, event: T) {
        self.subscribers
            .lock()
            .unwrap()
            .retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }
}

struct MpvActorInner {
    tx: Sender<ActorRequest>,
    events: Arc<EventHub<ListenEvent>>,
    session_events: Arc<EventHub<MpvSessionEvent>>,
    generation: Arc<AtomicU64>,
    load_id: Arc<AtomicU64>,
}

impl Drop for MpvActorInner {
    fn drop(&mut self) {
        let _ = self.tx.send(ActorRequest::Message(MpvMessage::Shutdown));
    }
}

#[derive(Clone)]
pub struct MpvActor {
    inner: Arc<MpvActorInner>,
}

enum ActorMode {
    Managed(MpvSessionOptions),
    Fixed,
}

enum ActorRequest {
    Message(MpvMessage),
    ConfigureProxy(ProxyConfig),
    ReplacePlaylist(Vec<String>),
    PlaylistAdd {
        url: String,
        index: i64,
    },
    PlaylistRemove(i64),
    PlaylistMove {
        from: i64,
        to: i64,
    },
    FrameImported {
        generation: u64,
        load_id: u64,
        transport: BufferTransport,
    },
    DmabufImportFailed {
        generation: u64,
        load_id: u64,
        fourcc: u32,
        modifier: u64,
        message: String,
    },
    VideoOutputInitializationFailed {
        generation: u64,
    },
    FirstFrameTimeout {
        generation: u64,
        load_id: u64,
    },
    Observed {
        generation: u64,
        event: ListenEvent,
        control: ObservedControl,
    },
}

#[derive(Clone, Copy)]
enum ObservedControl {
    None,
    StartFile,
    FileLoaded,
    Shutdown,
}

struct CandidateRuntime {
    mpv: Arc<Mpv>,
    generation: u64,
    candidate: Option<RendererCandidate>,
    proxy: Option<ProxyConnection>,
    ready: bool,
    file_started: bool,
    file_loaded: bool,
    resume_applied: bool,
    load_id: u64,
    consecutive_import_failures: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RendererSignature {
    vo: Option<String>,
    gpu_api: Option<String>,
    gpu_context: Option<String>,
}

impl RendererSignature {
    fn read(mpv: &Mpv) -> Self {
        Self {
            vo: mpv.get_property::<String>("options/vo").ok(),
            gpu_api: mpv.get_property::<String>("options/gpu-api").ok(),
            gpu_context: mpv.get_property::<String>("options/gpu-context").ok(),
        }
    }

    fn is_informative(&self) -> bool {
        self.vo.is_some() || self.gpu_api.is_some() || self.gpu_context.is_some()
    }

    fn matches_candidate(&self, candidate: RendererCandidate) -> bool {
        match candidate {
            RendererCandidate::VulkanDmabuf => {
                self.vo.as_deref() == Some("gpu-next")
                    && self.gpu_api.as_deref() == Some("vulkan")
                    && self.gpu_context.as_deref() == Some("waylandvk")
            }
            RendererCandidate::OpenGlDmabuf => {
                self.vo.as_deref() == Some("gpu-next")
                    && self.gpu_api.as_deref() == Some("opengl")
                    && self.gpu_context.as_deref() == Some("wayland")
            }
            RendererCandidate::WlShm => self.vo.as_deref() == Some("wlshm"),
        }
    }
}

impl CandidateRuntime {
    fn shutdown(&mut self) {
        let _ = self.mpv.command("quit", &[]);
        if let Some(mut proxy) = self.proxy.take() {
            proxy.stop();
        }
    }
}

#[derive(Clone, Default)]
struct PlaybackResume {
    playlist_pos: Option<i64>,
    position: Option<f64>,
    paused: Option<bool>,
    aid: Option<String>,
    sid: Option<String>,
    vid: Option<String>,
}

struct ActorState {
    mode: ActorMode,
    runtime: CandidateRuntime,
    proxy_config: Option<ProxyConfig>,
    candidates: Vec<RendererCandidate>,
    candidate_index: usize,
    playlist: Vec<String>,
    sticky_properties: HashMap<String, MpvValue>,
    pending_resume: Option<PlaybackResume>,
    renderer_forced: bool,
    events: Arc<EventHub<ListenEvent>>,
    session_events: Arc<EventHub<MpvSessionEvent>>,
    generation: Arc<AtomicU64>,
    load_id: Arc<AtomicU64>,
}

impl Default for MpvActor {
    fn default() -> Self {
        Self::new()
    }
}

impl MpvActor {
    pub fn new() -> Self {
        let options = DEFAULT_MPV_SESSION_OPTIONS
            .get()
            .cloned()
            .unwrap_or_default();
        Self::with_session_options(options).expect("Failed to create mpv instance")
    }

    pub fn with_session_options(options: MpvSessionOptions) -> libmpv2::Result<Self> {
        let candidates = options.fallback_policy.candidates();
        let candidates = if candidates.is_empty() {
            vec![RendererCandidate::WlShm]
        } else {
            candidates
        };
        let mut last_error = None;
        for index in 0..candidates.len() {
            let candidate = candidates[index];
            match create_managed_mpv(candidate) {
                Ok(mpv) => {
                    let signature = RendererSignature::read(&mpv);
                    let effective_candidate =
                        if signature.is_informative() && !signature.matches_candidate(candidate) {
                            candidates
                                .iter()
                                .copied()
                                .find(|candidate| signature.matches_candidate(*candidate))
                        } else {
                            Some(candidate)
                        };
                    let Some(effective_candidate) = effective_candidate else {
                        let _ = mpv.command("quit", &[]);
                        return Err(libmpv2::Error::Raw(MutsumiMpvError::MpvOptionError.code()));
                    };
                    let renderer_forced = effective_candidate != candidate;
                    return Ok(Self::spawn(
                        mpv,
                        Some(effective_candidate),
                        ActorMode::Managed(options),
                        candidates,
                        renderer_forced,
                    ));
                }
                Err(error) => {
                    tracing::warn!(
                        target: "mutsumi::mpv",
                        %candidate,
                        %error,
                        "candidate MPV initialization failed"
                    );
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.expect("candidate list is non-empty"))
    }

    pub fn with_initializer<F>(initializer: F) -> libmpv2::Result<Self>
    where
        F: FnOnce(libmpv2::MpvInitializer) -> libmpv2::Result<()>,
    {
        let mpv = Mpv::with_initializer(initializer)?;
        prepare_mpv(&mpv)?;
        Ok(Self::spawn(mpv, None, ActorMode::Fixed, Vec::new(), false))
    }

    pub fn new_libmpv() -> libmpv2::Result<Self> {
        Self::with_initializer(|mpv| {
            apply_library_defaults(&mpv)?;
            mpv.set_option("vo", "libmpv")?;
            if let Some(initializer) = MPV_INITIALIZER.get() {
                initializer(mpv)?;
            }
            Ok(())
        })
    }

    fn spawn(
        mpv: Mpv, candidate: Option<RendererCandidate>, mode: ActorMode,
        mut candidates: Vec<RendererCandidate>, renderer_forced: bool,
    ) -> Self {
        if let Some(candidate) = candidate
            && candidates.is_empty()
        {
            candidates.push(candidate);
        }

        let (tx, rx) = unbounded();
        let events = Arc::new(EventHub::default());
        let session_events = Arc::new(EventHub::default());
        let generation = Arc::new(AtomicU64::new(1));
        let load_id = Arc::new(AtomicU64::new(0));
        let mpv = Arc::new(mpv);
        spawn_event_loop(Arc::clone(&mpv), 1, tx.clone());
        let candidate_index = candidate
            .and_then(|candidate| candidates.iter().position(|item| *item == candidate))
            .unwrap_or(0);

        let state = ActorState {
            mode,
            runtime: CandidateRuntime {
                mpv,
                generation: 1,
                candidate,
                proxy: None,
                ready: false,
                file_started: false,
                file_loaded: false,
                resume_applied: false,
                load_id: 0,
                consecutive_import_failures: 0,
            },
            proxy_config: None,
            candidates,
            candidate_index,
            playlist: Vec::new(),
            sticky_properties: HashMap::new(),
            pending_resume: None,
            renderer_forced,
            events: Arc::clone(&events),
            session_events: Arc::clone(&session_events),
            generation: Arc::clone(&generation),
            load_id: Arc::clone(&load_id),
        };

        std::thread::Builder::new()
            .name("mutsumi mpv session".into())
            .spawn({
                let tx = tx.clone();
                move || run_actor(state, rx, tx)
            })
            .expect("Failed to spawn mpv actor thread");

        Self {
            inner: Arc::new(MpvActorInner {
                tx,
                events,
                session_events,
                generation,
                load_id,
            }),
        }
    }

    pub fn subscribe(&self) -> Receiver<ListenEvent> {
        self.inner.events.subscribe()
    }

    pub fn subscribe_session(&self) -> Receiver<MpvSessionEvent> {
        self.inner.session_events.subscribe()
    }

    pub fn current_generation(&self) -> u64 {
        self.inner.generation.load(Ordering::Acquire)
    }

    pub fn current_load_id(&self) -> u64 {
        self.inner.load_id.load(Ordering::Acquire)
    }

    pub(crate) fn configure_proxy(&self, config: ProxyConfig) {
        self.send(ActorRequest::ConfigureProxy(config));
    }

    pub(crate) fn replace_playlist(&self, urls: Vec<String>) {
        self.send(ActorRequest::ReplacePlaylist(urls));
    }

    pub(crate) fn playlist_add(&self, url: String, index: i64) {
        self.send(ActorRequest::PlaylistAdd { url, index });
    }

    pub(crate) fn playlist_remove(&self, index: i64) {
        self.send(ActorRequest::PlaylistRemove(index));
    }

    pub(crate) fn playlist_move(&self, from: i64, to: i64) {
        self.send(ActorRequest::PlaylistMove { from, to });
    }

    pub(crate) fn report_frame_imported(
        &self, generation: u64, load_id: u64, transport: BufferTransport,
    ) {
        self.send(ActorRequest::FrameImported {
            generation,
            load_id,
            transport,
        });
    }

    pub(crate) fn report_dmabuf_import_failed(
        &self, generation: u64, load_id: u64, fourcc: u32, modifier: u64, message: String,
    ) {
        self.send(ActorRequest::DmabufImportFailed {
            generation,
            load_id,
            fourcc,
            modifier,
            message,
        });
    }

    pub async fn mpv_handle(&self) -> Result<Arc<Mpv>, tokio::sync::oneshot::error::RecvError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.send(ActorRequest::Message(MpvMessage::InitRenderContext(tx)));
        rx.await
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<MpvValue>,
    {
        self.send(ActorRequest::Message(MpvMessage::SetProperty {
            property: property.to_owned(),
            value: value.into(),
        }));
    }

    pub async fn get_property(
        &self, property: &str, value_type: MpvValueType,
    ) -> Result<MpvValue, tokio::sync::oneshot::error::RecvError> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.send(ActorRequest::Message(MpvMessage::GetProperty {
            property: property.to_owned(),
            value_type,
            tx,
        }));
        rx.await
    }

    pub fn command(&self, cmd: &str, args: &[&str]) {
        self.send(ActorRequest::Message(MpvMessage::Command {
            cmd: cmd.to_owned(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        }));
    }

    pub fn shutdown(&self) {
        self.send(ActorRequest::Message(MpvMessage::Shutdown));
    }

    fn send(&self, request: ActorRequest) {
        if let Err(error) = self.inner.tx.send(request) {
            tracing::warn!(target: "mutsumi::mpv", %error, "mpv actor is unavailable");
        }
    }
}

fn apply_library_defaults(mpv: &libmpv2::MpvInitializer) -> libmpv2::Result<()> {
    mpv.set_option("input-default-bindings", "yes")?;
    mpv.set_option("keep-open", "yes")?;
    Ok(())
}

fn create_managed_mpv(candidate: RendererCandidate) -> libmpv2::Result<Mpv> {
    let mpv = Mpv::with_initializer(|mpv| {
        apply_library_defaults(&mpv)?;
        match candidate {
            RendererCandidate::VulkanDmabuf => {
                mpv.set_option("hwdec", "auto-safe")?;
                mpv.set_option("vo", "gpu-next")?;
                mpv.set_option("gpu-api", "vulkan")?;
                mpv.set_option("gpu-context", "waylandvk")?;
            }
            RendererCandidate::OpenGlDmabuf => {
                mpv.set_option("hwdec", "auto-safe")?;
                mpv.set_option("vo", "gpu-next")?;
                mpv.set_option("gpu-api", "opengl")?;
                mpv.set_option("gpu-context", "wayland")?;
            }
            RendererCandidate::WlShm => {
                mpv.set_option("hwdec", "auto-copy-safe")?;
                mpv.set_option("vo", "wlshm")?;
            }
        }

        if let Some(initializer) = MPV_INITIALIZER.get() {
            initializer(mpv)?;
        }
        Ok(())
    })?;
    prepare_mpv(&mpv)?;
    Ok(mpv)
}

fn prepare_mpv(mpv: &Mpv) -> libmpv2::Result<()> {
    logging::request_logs(mpv);
    tracing::debug!(target: "mutsumi::mpv", "mpv instance initialized");
    mpv.disable_deprecated_events()?;
    mpv.observe_property("duration", Format::Double, 0)?;
    mpv.observe_property("pause", Format::Flag, 1)?;
    mpv.observe_property("cache-speed", Format::Int64, 2)?;
    mpv.observe_property("track-list", Format::String, 3)?;
    mpv.observe_property("paused-for-cache", Format::Flag, 4)?;
    mpv.observe_property("demuxer-cache-time", Format::Int64, 5)?;
    mpv.observe_property("time-pos", Format::Int64, 6)?;
    mpv.observe_property("volume", Format::Int64, 7)?;
    mpv.observe_property("chapter-list", Format::String, 8)?;
    mpv.observe_property("speed", Format::Double, 9)?;
    mpv.observe_property("playlist", Format::String, 10)?;
    mpv.observe_property("eof-reached", Format::Flag, 11)?;
    Ok(())
}

fn spawn_event_loop(mpv: Arc<Mpv>, generation: u64, actor_tx: Sender<ActorRequest>) {
    let event_mpv = SendMpv {
        mpv,
        has_file: Cell::new(false),
        generation,
        actor_tx,
    };
    std::thread::Builder::new()
        .name(format!("mpv event loop {generation}"))
        .spawn(move || while event_mpv.handle_event() {})
        .expect("Failed to spawn mpv event thread");
}

fn run_actor(mut state: ActorState, rx: Receiver<ActorRequest>, tx: Sender<ActorRequest>) {
    while let Ok(request) = rx.recv() {
        match request {
            ActorRequest::Message(message) => {
                if !handle_message(&mut state, message) {
                    break;
                }
            }
            ActorRequest::ConfigureProxy(config) => {
                if state.runtime.file_loaded {
                    config.set_load_id(state.runtime.load_id);
                }
                state.proxy_config = Some(config);
            }
            ActorRequest::ReplacePlaylist(urls) => {
                state.playlist = urls;
                state.pending_resume = None;
                state.runtime.ready = false;
                state.runtime.file_started = false;
                state.runtime.file_loaded = false;
                state.runtime.resume_applied = false;
                state.runtime.load_id = state.runtime.load_id.wrapping_add(1);
                publish_load_id(&state);
                state.runtime.consecutive_import_failures = 0;
                if let Some(candidate) = state.runtime.candidate {
                    state
                        .session_events
                        .emit(MpvSessionEvent::CandidateStarted {
                            generation: state.runtime.generation,
                            candidate,
                        });
                }

                if state.playlist.is_empty() {
                    let _ = state.runtime.mpv.command("playlist-clear", &[]);
                    let _ = state.runtime.mpv.command("stop", &[]);
                } else {
                    match start_playlist(&mut state) {
                        Ok(()) => start_first_frame_watchdog(&state, tx.clone()),
                        Err(reason) => fallback(&mut state, reason, &tx),
                    }
                }
            }
            ActorRequest::PlaylistAdd { url, index } => {
                let insertion = usize::try_from(index)
                    .ok()
                    .map_or(state.playlist.len(), |index| {
                        index.min(state.playlist.len())
                    });
                state.playlist.insert(insertion, url.clone());
                let _ = state.runtime.mpv.command(
                    "loadfile",
                    &[url.as_str(), "insert-at", index.to_string().as_str()],
                );
            }
            ActorRequest::PlaylistRemove(index) => {
                if let Ok(index_usize) = usize::try_from(index)
                    && index_usize < state.playlist.len()
                {
                    state.playlist.remove(index_usize);
                }
                let _ = state
                    .runtime
                    .mpv
                    .command("playlist-remove", &[index.to_string().as_str()]);
            }
            ActorRequest::PlaylistMove { from, to } => {
                if let (Ok(from_index), Ok(to_index)) = (usize::try_from(from), usize::try_from(to))
                    && from_index < state.playlist.len()
                {
                    let entry = state.playlist.remove(from_index);
                    state
                        .playlist
                        .insert(to_index.min(state.playlist.len()), entry);
                }
                let _ = state.runtime.mpv.command(
                    "playlist-move",
                    &[from.to_string().as_str(), to.to_string().as_str()],
                );
            }
            ActorRequest::FrameImported {
                generation,
                load_id,
                transport,
            } => {
                if generation == state.runtime.generation
                    && load_id == state.runtime.load_id
                    && state.runtime.file_loaded
                {
                    let expected = state.runtime.candidate.map(RendererCandidate::transport);
                    if expected == Some(transport) {
                        state.runtime.consecutive_import_failures = 0;
                        mark_ready(&mut state);
                    } else if let Some(expected) = expected {
                        fallback(
                            &mut state,
                            CandidateFailure::UnexpectedTransport {
                                expected,
                                actual: transport,
                            },
                            &tx,
                        );
                    }
                }
            }
            ActorRequest::DmabufImportFailed {
                generation,
                load_id,
                fourcc,
                modifier,
                message,
            } => {
                if generation != state.runtime.generation
                    || load_id != state.runtime.load_id
                    || !state.runtime.file_loaded
                {
                    continue;
                }
                state.runtime.consecutive_import_failures += 1;
                if !state.runtime.ready || state.runtime.consecutive_import_failures >= 3 {
                    fallback(
                        &mut state,
                        CandidateFailure::DmabufImport {
                            fourcc,
                            modifier,
                            message,
                        },
                        &tx,
                    );
                }
            }
            ActorRequest::VideoOutputInitializationFailed { generation } => {
                if generation == state.runtime.generation {
                    fallback(&mut state, CandidateFailure::VideoOutputInitialization, &tx);
                }
            }
            ActorRequest::FirstFrameTimeout {
                generation,
                load_id,
            } => {
                if generation == state.runtime.generation
                    && load_id == state.runtime.load_id
                    && !state.runtime.ready
                {
                    fallback(&mut state, CandidateFailure::FirstFrameTimeout, &tx);
                }
            }
            ActorRequest::Observed {
                generation,
                event,
                control,
            } => {
                if generation != state.runtime.generation {
                    continue;
                }
                state.events.emit(event);
                match control {
                    ObservedControl::None => {}
                    ObservedControl::StartFile => {
                        if state.runtime.file_started {
                            state.runtime.load_id = state.runtime.load_id.wrapping_add(1);
                            publish_load_id(&state);
                            state.runtime.ready = false;
                            state.runtime.resume_applied = false;
                        }
                        state.runtime.file_started = true;
                        state.runtime.file_loaded = false;
                        start_first_frame_watchdog(&state, tx.clone());
                    }
                    ObservedControl::FileLoaded => {
                        if restore_playback(&mut state) {
                            state.runtime.file_loaded = true;
                            activate_proxy_load(&state);
                            if !has_selected_video(&state.runtime.mpv) {
                                mark_ready(&mut state);
                            }
                        }
                    }
                    ObservedControl::Shutdown => break,
                }
            }
        }
    }

    state.runtime.shutdown();
}

fn handle_message(state: &mut ActorState, message: MpvMessage) -> bool {
    match message {
        MpvMessage::Command { cmd, args } => {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let _ = state.runtime.mpv.command(&cmd, &args);
        }
        MpvMessage::SetProperty { property, value } => {
            if matches!(property.as_str(), "vo" | "gpu-api" | "gpu-context") {
                state.renderer_forced = true;
            }
            state
                .sticky_properties
                .insert(property.clone(), value.clone());
            let _ = value.set_on(&state.runtime.mpv, &property);
        }
        MpvMessage::GetProperty {
            property,
            value_type,
            tx,
        } => {
            if let Ok(value) = get_property_value(&state.runtime.mpv, &property, value_type) {
                let _ = tx.send(value);
            }
        }
        MpvMessage::InitRenderContext(tx) => {
            let _ = tx.send(Arc::clone(&state.runtime.mpv));
        }
        MpvMessage::Shutdown => {
            state.events.emit(ListenEvent::Shutdown);
            return false;
        }
    }
    true
}

fn start_playlist(state: &mut ActorState) -> Result<(), CandidateFailure> {
    let Some(first) = state.playlist.first() else {
        return Ok(());
    };

    if state.runtime.proxy.is_none()
        && let (Some(config), Some(candidate)) = (&state.proxy_config, state.runtime.candidate)
    {
        if candidate.transport() == BufferTransport::Dmabuf && !config.supports_dmabuf() {
            return Err(CandidateFailure::Proxy(
                "the local GDK consumer did not report any importable DMA-BUF formats".into(),
            ));
        }
        let mut connection = config
            .start(state.runtime.generation, candidate.transport())
            .map_err(|error| CandidateFailure::Proxy(error.to_string()))?;
        let client_fd = connection.take_client_fd().ok_or_else(|| {
            CandidateFailure::Proxy("proxy did not return a client file descriptor".into())
        })?;
        command_with_wayland_socket(
            &state.runtime.mpv,
            &connection,
            client_fd,
            "loadfile",
            &[first, "replace"],
        )?;
        state.runtime.proxy = Some(connection);
    } else {
        state
            .runtime
            .mpv
            .command("loadfile", &[first, "replace"])
            .map_err(|error| CandidateFailure::Unavailable(error.to_string()))?;
    }

    for url in state.playlist.iter().skip(1) {
        state
            .runtime
            .mpv
            .command("loadfile", &[url, "append"])
            .map_err(|error| CandidateFailure::Unavailable(error.to_string()))?;
    }
    Ok(())
}

fn command_with_wayland_socket(
    mpv: &Mpv, connection: &ProxyConnection, client_fd: std::os::fd::OwnedFd, command: &str,
    args: &[&str],
) -> Result<(), CandidateFailure> {
    // MPV currently has no per-instance Wayland socket option. Keep the
    // process environment handoff serialized until this exact synthetic
    // connection requests its registry; returning from `loadfile` alone does
    // not prove that MPV's VO thread has consumed the descriptor.
    let _guard = WAYLAND_HANDOFF_LOCK.lock().unwrap();
    let previous = std::env::var_os("WAYLAND_SOCKET");
    let raw_fd = client_fd.into_raw_fd();
    unsafe { std::env::set_var("WAYLAND_SOCKET", raw_fd.to_string()) };

    let command_result = mpv
        .command(command, args)
        .map_err(|error| CandidateFailure::Unavailable(error.to_string()));
    let result = command_result.and_then(|()| {
        connection
            .wait_until_connected(std::time::Duration::from_secs(10))
            .map_err(|error| CandidateFailure::Proxy(error.to_string()))
    });

    match previous {
        Some(previous) => unsafe { std::env::set_var("WAYLAND_SOCKET", previous) },
        None => unsafe { std::env::remove_var("WAYLAND_SOCKET") },
    }
    if matches!(result, Err(CandidateFailure::Unavailable(_))) {
        unsafe { libc::close(raw_fd) };
    } else if result.is_err() {
        // The client may already own the descriptor even if it did not reach
        // get_registry before the timeout. The retiring MPV candidate will
        // close it during shutdown; closing it here could race with libwayland.
        tracing::warn!(
            raw_fd,
            "leaving ambiguous Wayland fd ownership to the retiring MPV candidate"
        );
    }
    result
}

fn fallback(state: &mut ActorState, mut reason: CandidateFailure, tx: &Sender<ActorRequest>) {
    let ActorMode::Managed(_) = &state.mode else {
        state
            .session_events
            .emit(MpvSessionEvent::Failed { reason });
        return;
    };

    if state.renderer_forced {
        let override_reason = CandidateFailure::UserOverride(reason.to_string());
        if let Some(candidate) = state.runtime.candidate {
            state.session_events.emit(MpvSessionEvent::CandidateFailed {
                generation: state.runtime.generation,
                candidate,
                reason,
                will_retry: false,
            });
        }
        state.session_events.emit(MpvSessionEvent::Failed {
            reason: override_reason.clone(),
        });
        state
            .events
            .emit(ListenEvent::Error(override_reason.to_string()));
        return;
    }

    if state.pending_resume.is_none() && state.runtime.file_started {
        state.pending_resume = Some(capture_playback(&state.runtime.mpv));
    }

    loop {
        let next_index = state.candidate_index + 1;
        let will_retry = next_index < state.candidates.len();
        if let Some(candidate) = state.runtime.candidate {
            state.session_events.emit(MpvSessionEvent::CandidateFailed {
                generation: state.runtime.generation,
                candidate,
                reason: reason.clone(),
                will_retry,
            });
        }

        if !will_retry {
            state.session_events.emit(MpvSessionEvent::Failed {
                reason: reason.clone(),
            });
            state.events.emit(ListenEvent::Error(reason.to_string()));
            return;
        }

        state.runtime.shutdown();
        state.candidate_index = next_index;
        let candidate = state.candidates[next_index];
        let generation = state.runtime.generation.wrapping_add(1);
        let mpv = match create_managed_mpv(candidate) {
            Ok(mpv) => Arc::new(mpv),
            Err(error) => {
                let failure = CandidateFailure::Initialization(error.to_string());
                let will_retry = next_index + 1 < state.candidates.len();
                state.session_events.emit(MpvSessionEvent::CandidateFailed {
                    generation,
                    candidate,
                    reason: failure.clone(),
                    will_retry,
                });
                if !will_retry {
                    state.session_events.emit(MpvSessionEvent::Failed {
                        reason: failure.clone(),
                    });
                    state.events.emit(ListenEvent::Error(failure.to_string()));
                    return;
                }
                state.runtime.candidate = None;
                reason = failure;
                continue;
            }
        };
        let renderer_signature = RendererSignature::read(&mpv);
        if renderer_signature.is_informative() && !renderer_signature.matches_candidate(candidate) {
            let failure = CandidateFailure::UserOverride(format!(
                "effective renderer options {renderer_signature:?} do not match {candidate}"
            ));
            state.session_events.emit(MpvSessionEvent::Failed {
                reason: failure.clone(),
            });
            state.events.emit(ListenEvent::Error(failure.to_string()));
            let _ = mpv.command("quit", &[]);
            return;
        }
        spawn_event_loop(Arc::clone(&mpv), generation, tx.clone());
        state.runtime = CandidateRuntime {
            mpv,
            generation,
            candidate: Some(candidate),
            proxy: None,
            ready: false,
            file_started: false,
            file_loaded: false,
            resume_applied: false,
            load_id: state.runtime.load_id.wrapping_add(1),
            consecutive_import_failures: 0,
        };
        state.generation.store(generation, Ordering::Release);
        publish_load_id(state);
        state
            .session_events
            .emit(MpvSessionEvent::CandidateStarted {
                generation,
                candidate,
            });
        replay_sticky_properties(state);
        let _ = state.runtime.mpv.set_property("pause", true);

        match start_playlist(state) {
            Ok(()) => {
                start_first_frame_watchdog(state, tx.clone());
                return;
            }
            Err(next_reason) => reason = next_reason,
        }
    }
}

fn capture_playback(mpv: &Mpv) -> PlaybackResume {
    PlaybackResume {
        playlist_pos: mpv
            .get_property::<i64>("playlist-pos")
            .ok()
            .filter(|position| *position >= 0),
        position: mpv.get_property::<f64>("time-pos").ok(),
        paused: mpv.get_property::<bool>("pause").ok(),
        aid: mpv.get_property::<String>("aid").ok(),
        sid: mpv.get_property::<String>("sid").ok(),
        vid: mpv.get_property::<String>("vid").ok(),
    }
}

fn replay_sticky_properties(state: &ActorState) {
    for (property, value) in &state.sticky_properties {
        if matches!(
            property.as_str(),
            "time-pos" | "pause" | "aid" | "sid" | "vid"
        ) {
            continue;
        }
        if let Err(error) = value.set_on(&state.runtime.mpv, property) {
            tracing::debug!(target: "mutsumi::mpv", %property, %error, "could not restore property");
        }
    }
}

fn restore_playback(state: &mut ActorState) -> bool {
    let Some(resume) = state.pending_resume.clone() else {
        return true;
    };
    if state.runtime.resume_applied {
        return true;
    }

    if let Some(target) = resume.playlist_pos {
        let current = state.runtime.mpv.get_property::<i64>("playlist-pos").ok();
        if current != Some(target) {
            state.runtime.file_started = false;
            state.runtime.file_loaded = false;
            state.runtime.ready = false;
            state.runtime.load_id = state.runtime.load_id.wrapping_add(1);
            publish_load_id(state);
            let _ = state.runtime.mpv.set_property("playlist-pos", target);
            return false;
        }
    }
    if let Some(vid) = resume.vid.as_deref() {
        let _ = state.runtime.mpv.set_property("vid", vid);
    }
    if let Some(aid) = resume.aid.as_deref() {
        let _ = state.runtime.mpv.set_property("aid", aid);
    }
    if let Some(sid) = resume.sid.as_deref() {
        let _ = state.runtime.mpv.set_property("sid", sid);
    }
    if let Some(position) = resume.position {
        let position = position.to_string();
        let _ = state
            .runtime
            .mpv
            .command("seek", &[position.as_str(), "absolute+exact"]);
    }
    state.runtime.resume_applied = true;
    true
}

fn has_selected_video(mpv: &Mpv) -> bool {
    if mpv
        .get_property::<String>("video-format")
        .is_ok_and(|format| !format.is_empty())
    {
        return true;
    }

    let Ok(track_list) = mpv.get_property::<String>("track-list") else {
        return false;
    };
    serde_json::from_str::<Value>(&track_list)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .is_some_and(|tracks| {
            tracks.iter().any(|track| {
                track.get("type").and_then(Value::as_str) == Some("video")
                    && track.get("selected").and_then(Value::as_bool) != Some(false)
            })
        })
}

fn publish_load_id(state: &ActorState) {
    state
        .load_id
        .store(state.runtime.load_id, Ordering::Release);
}

fn activate_proxy_load(state: &ActorState) {
    if let Some(config) = &state.proxy_config {
        config.set_load_id(state.runtime.load_id);
    }
}

fn start_first_frame_watchdog(state: &ActorState, tx: Sender<ActorRequest>) {
    let ActorMode::Managed(options) = &state.mode else {
        return;
    };
    let timeout = options.first_frame_timeout;
    let generation = state.runtime.generation;
    let load_id = state.runtime.load_id;
    std::thread::spawn(move || {
        std::thread::sleep(timeout);
        let _ = tx.send(ActorRequest::FirstFrameTimeout {
            generation,
            load_id,
        });
    });
}

fn mark_ready(state: &mut ActorState) {
    if state.runtime.ready {
        return;
    }
    state.runtime.ready = true;
    if let Some(resume) = state.pending_resume.take()
        && let Some(paused) = resume.paused
    {
        let _ = state.runtime.mpv.set_property("pause", paused);
    }
    if let Some(candidate) = state.runtime.candidate {
        state.session_events.emit(MpvSessionEvent::Ready {
            generation: state.runtime.generation,
            candidate,
        });
    }
}

impl Deref for SendMpv {
    type Target = Mpv;

    fn deref(&self) -> &Self::Target {
        &self.mpv
    }
}

impl SendMpv {
    fn emit(&self, event: ListenEvent) {
        let _ = self.actor_tx.send(ActorRequest::Observed {
            generation: self.generation,
            event,
            control: ObservedControl::None,
        });
    }

    fn emit_control(&self, event: ListenEvent, control: ObservedControl) {
        let _ = self.actor_tx.send(ActorRequest::Observed {
            generation: self.generation,
            event,
            control,
        });
    }

    // Blocks until the next event. Returns false once mpv shuts down.
    fn handle_event(&self) -> bool {
        let Some(event) = self.wait_event(1.0) else {
            return true;
        };

        match event {
            Ok(event) => match event {
                Event::LogMessage {
                    prefix,
                    text,
                    log_level,
                    ..
                } => logging::emit_log(prefix, log_level, text),
                Event::PropertyChange { name, change, .. } => match name {
                    "duration" => {
                        if let PropertyData::Double(duration) = change {
                            self.emit(ListenEvent::Duration(duration));
                        }
                    }
                    "pause" => {
                        if let PropertyData::Flag(pause) = change
                            && (self.has_file.get() || pause)
                        {
                            self.emit(ListenEvent::Pause(pause));
                        }
                    }
                    "cache-speed" => {
                        if let PropertyData::Int64(speed) = change {
                            self.emit(ListenEvent::CacheSpeed(speed));
                        }
                    }
                    "track-list" => {
                        if let PropertyData::Str(node) = change {
                            self.emit(ListenEvent::TrackList(node_to_tracks(node)));
                        }
                    }
                    "chapter-list" => {
                        if let PropertyData::Str(node) = change {
                            self.emit(ListenEvent::ChapterList(node_to_chapter_list(node)));
                        }
                    }
                    "playlist" => {
                        if let PropertyData::Str(node) = change {
                            self.emit(ListenEvent::Playlist(node_to_playlist(node)));
                        }
                    }
                    "volume" => {
                        if let PropertyData::Int64(volume) = change {
                            self.emit(ListenEvent::Volume(volume));
                        }
                    }
                    "speed" => {
                        if let PropertyData::Double(speed) = change {
                            self.emit(ListenEvent::Speed(speed));
                        }
                    }
                    "demuxer-cache-time" => {
                        if let PropertyData::Int64(time) = change {
                            self.emit(ListenEvent::DemuxerCacheTime(time));
                        }
                    }
                    "time-pos" => {
                        if let PropertyData::Int64(time) = change {
                            self.emit(ListenEvent::TimePos(time));
                        }
                    }
                    "eof-reached" => {
                        if let PropertyData::Flag(true) = change {
                            self.emit(ListenEvent::PlaybackEnded);
                        }
                    }
                    "paused-for-cache" => {
                        if let PropertyData::Flag(pause) = change {
                            let seeking = self.get_property::<bool>("seeking").unwrap_or(false);
                            let time_millis =
                                self.get_property::<f64>("audio-pts").unwrap_or(0.0) * 1000.0;
                            self.emit(ListenEvent::PausedForCache(pause || seeking, time_millis));
                        }
                    }
                    _ => {}
                },
                Event::Seek => {
                    let time_millis = self.get_property::<f64>("audio-pts").unwrap_or(0.0) * 1000.0;
                    self.emit(ListenEvent::Seek(time_millis));
                }
                Event::PlaybackRestart => {
                    let time_millis = self.get_property::<f64>("audio-pts").unwrap_or(0.0) * 1000.0;
                    self.emit(ListenEvent::PlaybackRestart(time_millis));
                }
                Event::FileLoaded => {
                    self.emit_control(ListenEvent::FileLoaded, ObservedControl::FileLoaded);
                }
                Event::EndFile(reason) => {
                    self.has_file.set(false);
                    self.emit(ListenEvent::Eof(reason));
                }
                Event::StartFile => {
                    self.has_file.set(true);
                    self.emit_control(ListenEvent::StartFile, ObservedControl::StartFile);
                    let pause = self.get_property::<bool>("pause").unwrap_or(false);
                    self.emit(ListenEvent::Pause(pause));
                }
                Event::Shutdown => {
                    self.emit_control(ListenEvent::Shutdown, ObservedControl::Shutdown);
                    return false;
                }
                _ => {}
            },
            Err(error) => {
                let libmpv2::Error::Raw(code) = error else {
                    return true;
                };
                if MutsumiMpvError::from_code(code) == MutsumiMpvError::MpvVoInitFailed {
                    let _ = self
                        .actor_tx
                        .send(ActorRequest::VideoOutputInitializationFailed {
                            generation: self.generation,
                        });
                } else {
                    self.emit(ListenEvent::Error(
                        MutsumiMpvError::from_code(code).to_string(),
                    ));
                }
            }
        }
        true
    }
}

fn get_property_value(
    mpv: &Mpv, property: &str, value_type: MpvValueType,
) -> libmpv2::Result<MpvValue> {
    match value_type {
        MpvValueType::Bool => mpv.get_property::<bool>(property).map(MpvValue::Bool),
        MpvValueType::I64 => mpv.get_property::<i64>(property).map(MpvValue::I64),
        MpvValueType::F64 => mpv.get_property::<f64>(property).map(MpvValue::F64),
        MpvValueType::String => mpv.get_property::<String>(property).map(MpvValue::String),
    }
}

fn node_to_chapter_list(value: &str) -> ChapterList {
    let mut chapters = Vec::new();

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return ChapterList(chapters);
    };
    let Some(array) = json.as_array() else {
        return ChapterList(chapters);
    };

    for node in array {
        let Some(obj) = node.as_object() else {
            continue;
        };

        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let time = obj
            .get("time")
            .and_then(Value::as_f64)
            .or_else(|| {
                obj.get("time")
                    .and_then(Value::as_i64)
                    .map(|value| value as f64)
            })
            .unwrap_or(0.0);

        chapters.push(Chapter { title, time });
    }

    ChapterList(chapters)
}

#[derive(Debug, Clone)]
pub struct ChapterList(pub Vec<Chapter>);

impl IntoIterator for ChapterList {
    type Item = Chapter;
    type IntoIter = std::vec::IntoIter<Chapter>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub time: f64,
}

#[derive(Debug, Clone)]
pub struct PlaylistEntry {
    pub filename: String,
    pub title: String,
    pub current: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Playlist(pub Vec<PlaylistEntry>);

impl IntoIterator for Playlist {
    type Item = PlaylistEntry;
    type IntoIter = std::vec::IntoIter<PlaylistEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

fn node_to_playlist(value: &str) -> Playlist {
    let mut entries = Vec::new();

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return Playlist(entries);
    };
    let Some(array) = json.as_array() else {
        return Playlist(entries);
    };

    for node in array {
        let Some(obj) = node.as_object() else {
            continue;
        };

        let filename = obj
            .get("filename")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let current = obj.get("current").and_then(Value::as_bool).unwrap_or(false);

        entries.push(PlaylistEntry {
            filename,
            title,
            current,
        });
    }

    Playlist(entries)
}

#[derive(Debug, Clone)]
pub struct MpvTrack {
    pub id: i64,
    pub title: String,
    pub lang: String,
    pub type_: String,
}

#[derive(Debug, Clone)]
pub struct DanmakuTrack {
    pub external_url: String,
}

#[derive(Debug, Clone)]
pub struct MpvTracks {
    pub audio_tracks: Vec<MpvTrack>,
    pub sub_tracks: Vec<MpvTrack>,
    pub danmaku_track: Option<DanmakuTrack>,
}

fn node_to_tracks(value: &str) -> MpvTracks {
    let mut audio_tracks = Vec::new();
    let mut sub_tracks = Vec::new();
    let mut danmaku_track = None;

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return MpvTracks {
            audio_tracks,
            sub_tracks,
            danmaku_track,
        };
    };
    let Some(array) = json.as_array() else {
        return MpvTracks {
            audio_tracks,
            sub_tracks,
            danmaku_track,
        };
    };

    for node in array {
        let Some(obj) = node.as_object() else {
            continue;
        };

        let id = obj.get("id").and_then(Value::as_i64).unwrap_or(0);
        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let lang = obj
            .get("lang")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let type_ = obj
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        if type_ == "sub" && lang == "danmaku" {
            let external = obj
                .get("external")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !external {
                continue;
            }

            let external_filename = obj
                .get("external-filename")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let Some(external) =
                external_filename.strip_prefix("edl://!no_clip;!delay_open,media_type=sub;%44%")
            else {
                continue;
            };
            danmaku_track = Some(DanmakuTrack {
                external_url: external.to_string(),
            });
            continue;
        }

        let track = MpvTrack {
            id,
            title,
            lang,
            type_,
        };
        if track.type_ == "audio" {
            audio_tracks.push(track);
        } else if track.type_ == "sub" {
            sub_tracks.push(track);
        }
    }

    MpvTracks {
        audio_tracks,
        sub_tracks,
        danmaku_track,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signature(vo: &str, gpu_api: &str, gpu_context: &str) -> RendererSignature {
        RendererSignature {
            vo: Some(vo.to_owned()),
            gpu_api: Some(gpu_api.to_owned()),
            gpu_context: Some(gpu_context.to_owned()),
        }
    }

    #[test]
    fn effective_renderer_signature_identifies_candidates() {
        assert!(
            signature("gpu-next", "vulkan", "waylandvk")
                .matches_candidate(RendererCandidate::VulkanDmabuf)
        );
        assert!(
            signature("gpu-next", "opengl", "wayland")
                .matches_candidate(RendererCandidate::OpenGlDmabuf)
        );
        assert!(signature("wlshm", "auto", "auto").matches_candidate(RendererCandidate::WlShm));
    }

    #[test]
    fn partial_renderer_override_does_not_masquerade_as_another_candidate() {
        let overridden = signature("gpu-next", "vulkan", "waylandvk");
        assert!(!overridden.matches_candidate(RendererCandidate::OpenGlDmabuf));
        assert!(!overridden.matches_candidate(RendererCandidate::WlShm));
    }
}
