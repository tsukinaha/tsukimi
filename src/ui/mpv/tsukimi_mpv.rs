use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc,
        atomic::AtomicU32,
    },
    thread::JoinHandle,
};

use libmpv2::{
    GetData,
    Mpv,
    SetData,
    events::{
        EventContext,
        PropertyData,
    },
    mpv_node::MpvNode,
    render::RenderContext,
};
use tracing::{
    info,
    warn,
};

#[derive(Debug)]
pub struct MpvTrack {
    pub id: i64,
    pub title: String,
    pub lang: String,
    pub type_: String,
}

pub struct TsukimiMPV {
    pub mpv: Arc<Mpv>,
    pub ctx: RefCell<Option<RenderContext>>,
    pub event_thread_alive: Arc<AtomicU32>,
}

impl std::fmt::Debug for TsukimiMPV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TsukimiMPV").finish()
    }
}

pub enum TrackSelection {
    Track(i64),
    None,
}

impl std::fmt::Display for TrackSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            TrackSelection::Track(id) => id.to_string(),
            TrackSelection::None => "no".to_string(),
        };
        write!(f, "{}", str)
    }
}

pub const PAUSED: u32 = 0;
pub const ACTIVE: u32 = 1;
pub const SHUTDOWN: u32 = 2;

impl Default for TsukimiMPV {
    fn default() -> Self {
        unsafe {
            use libc::{
                LC_NUMERIC,
                setlocale,
            };
            setlocale(LC_NUMERIC, c"C".as_ptr() as *const _);
        }

        #[cfg(target_os = "macos")]
        let library = unsafe { libloading::os::unix::Library::new("libepoxy.0.dylib") }.unwrap();
        #[cfg(all(unix, not(target_os = "macos")))]
        let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }.unwrap();
        #[cfg(windows)]
        let library = libloading::os::windows::Library::open_already_loaded("libepoxy-0.dll")
            .or_else(|_| libloading::os::windows::Library::open_already_loaded("epoxy-0.dll"))
            .unwrap();

        epoxy::load_with(|name| {
            unsafe { library.get::<_>(name.as_bytes()) }
                .map(|symbol| *symbol)
                .unwrap_or(std::ptr::null())
        });

        gl::load_with(|name| epoxy::get_proc_addr(name) as *const _);

        let mpv = Mpv::with_initializer(|init| {
            if SETTINGS.mpv_config() {
                init.set_property("config", true)?;
                init.set_property("config-dir", SETTINGS.mpv_config_dir())?;
            }
            init.set_property("input-vo-keyboard", true)?;
            init.set_property("input-default-bindings", true)?;
            init.set_property("user-agent", crate::USER_AGENT.as_str())?;
            init.set_property("video-timing-offset", 0)?;
            init.set_property("video-sync", "audio")?;
            match SETTINGS.mpv_video_output() {
                0 => {
                    init.set_property("vo", "libmpv")?;
                    init.set_property("osc", false)?;
                    init.set_property("osd-level", 0)?;
                }
                1 => init.set_property("vo", "gpu-next")?,
                2 => init.set_property("vo", "dmabuf-wayland")?,
                _ => unreachable!(),
            }
            init.set_property(
                "demuxer-max-bytes",
                format!("{}MiB", SETTINGS.mpv_cache_size()),
            )?;
            init.set_property("cache-secs", (SETTINGS.mpv_cache_time()) as i64)?;
            init.set_property("volume", SETTINGS.mpv_default_volume() as i64)?;
            init.set_property("sub-font-size", SETTINGS.mpv_subtitle_size() as i64)?;
            init.set_property("sub-font", SETTINGS.mpv_subtitle_font())?;
            init.set_property("sub-scale", SETTINGS.mpv_subtitle_scale())?;
            init.set_property("hwdec", match_hwdec_interop(SETTINGS.mpv_hwdec()))?;
            init.set_property("scale", match_video_upscale(SETTINGS.mpv_video_scale()))?;
            if SETTINGS.mpv_action_after_video_end() == 1 {
                init.set_property("loop", "inf")?;
            } else {
                init.set_property("loop", "no")?;
            }
            init.set_property(
                "audio-channels",
                match_audio_channels(SETTINGS.mpv_audio_channel()),
            )?;
            if let Some(uri) = crate::client::proxy::get_proxy_settings() {
                let url = Url::parse(&uri)
                    .map_or_else(|_| format!("http://{}", uri), |_| uri.to_string());
                init.set_property("http-proxy", url)?;
            };
            match SETTINGS.mpv_audio_preferred_lang() {
                0 => init.set_property("alang", "")?,
                1 => init.set_property("alang", "eng")?,
                2 => init.set_property("alang", "chs")?,
                3 => init.set_property("alang", "jpn")?,
                4 => init.set_property("alang", "chi")?,
                5 => init.set_property("alang", "ara")?,
                6 => init.set_property("alang", "nob")?,
                7 => init.set_property("alang", "por")?,
                8 => init.set_property("alang", "fre")?,
                _ => unreachable!(),
            }
            Ok(())
        })
        .expect("Failed to create mpv instance");

        Self {
            mpv: Arc::new(mpv),
            ctx: RefCell::new(None),
            event_thread_alive: Arc::new(AtomicU32::new(PAUSED)),
        }
    }
}

use flume::{
    Receiver,
    Sender,
    unbounded,
};
use libmpv2::events::Event;
use once_cell::sync::Lazy;

pub struct RenderUpdate {
    pub tx: Sender<bool>,
    pub rx: Receiver<bool>,
}

// Give render update a unique channel
pub static RENDER_UPDATE: Lazy<RenderUpdate> = Lazy::new(|| {
    let (tx, rx) = unbounded::<bool>();

    RenderUpdate { tx, rx }
});

pub struct MPVEventChannel {
    pub tx: Sender<ListenEvent>,
    pub rx: Receiver<ListenEvent>,
}

pub enum ListenEvent {
    Seek,
    PlaybackRestart,
    Eof(u32),
    StartFile,
    Duration(f64),
    Pause(bool),
    CacheSpeed(i64),
    Error(String),
    TrackList(MpvTracks),
    Volume(i64),
    Speed(f64),
    Shutdown,
    DemuxerCacheTime(i64),
    TimePos(i64),
    PausedForCache(bool),
    ChapterList(ChapterList),
}

pub static MPV_EVENT_CHANNEL: Lazy<MPVEventChannel> = Lazy::new(|| {
    let (tx, rx) = unbounded::<ListenEvent>();

    MPVEventChannel { tx, rx }
});

impl TsukimiMPV {
    pub fn set_position(&self, value: f64) {
        self.set_property("time-pos", value);
    }

    pub fn set_percent_position(&self, value: f64) {
        self.set_property("percent-pos", value);
    }

    pub fn position(&self) -> f64 {
        self.get_property("time-pos").unwrap_or(0.0)
    }

    pub fn paused(&self) -> bool {
        self.get_property("pause").unwrap_or(true)
    }

    pub fn pause(&self, pause: bool) {
        self.set_property("pause", pause);
    }

    pub fn command_pause(&self) {
        self.command("cycle", &["pause"]);
    }

    pub fn add_sub(&self, url: &str) {
        self.command("sub-add", &[url, "select"]);
    }

    pub fn load_video(&self, url: &str) {
        self.command("loadfile", &[url, "replace"]);
    }

    pub fn set_start(&self, percentage: f64) {
        self.set_property("start", format!("{}%", percentage as u32));
    }

    pub fn set_volume(&self, volume: i64) {
        self.set_property("volume", volume);
    }

    pub fn volume_scroll(&self, value: i64) {
        self.command("add", &["volume", &value.to_string()]);
    }

    pub fn set_speed(&self, speed: f64) {
        self.set_property("speed", speed);
    }

    pub fn set_aid(&self, aid: TrackSelection) {
        self.set_property("aid", aid.to_string());
    }

    pub fn set_sid(&self, sid: TrackSelection) {
        self.set_property("sid", sid.to_string());
    }

    pub fn seek_forward(&self, value: i64) {
        self.command("seek", &[&value.to_string()]);
    }

    pub fn seek_backward(&self, value: i64) {
        self.command("seek", &[&(-value).to_string()]);
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        let keystr = get_full_keystr(key, state);
        if let Some(keystr) = keystr {
            info!("MPV Catch Key pressed: {}", keystr);
            self.command("keypress", &[&keystr]);
        }
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        let keystr = get_full_keystr(key, state);
        if let Some(keystr) = keystr {
            info!("MPV Catch Key released: {}", keystr);
            self.command("keyup", &[&keystr]);
        }
    }

    pub fn stop(&self) {
        self.command("stop", &[]);
    }

    pub fn display_stats_toggle(&self) {
        self.command("script-binding", &["stats/display-stats-toggle"]);
    }

    pub fn set_slang(&self, value: String) {
        self.set_property("slang", value);
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: SetData + Send + 'static,
    {
        let mpv = Arc::clone(&self.mpv);
        let property = property.to_string();
        spawn_tokio_without_await(async move {
            mpv.set_property(&property, value)
                .map_err(|e| warn!("MPV set property Error: {}, Property: {}", e, property))
                .ok();
        });
    }

    fn get_property<V>(&self, property: &str) -> Option<V>
    where
        V: GetData,
    {
        let mpv = Arc::clone(&self.mpv);
        mpv.get_property(property).ok()
    }

    fn command(&self, cmd: &str, args: &[&str]) {
        let mpv = Arc::clone(&self.mpv);
        let cmd = cmd.to_string();
        let args = args.iter().map(|&arg| arg.to_string()).collect::<Vec<_>>();
        spawn_tokio_without_await(async move {
            let args_ref: Vec<&str> = args.iter().map(|arg| arg.as_str()).collect();
            mpv.command(&cmd, &args_ref)
                .map_err(|e| warn!("MPV command Error: {}, Command: {}", e, cmd))
                .ok();
        });
    }

    pub fn get_track_id(&self, type_: &str) -> i64 {
        let Some(track) = self.get_property::<String>(type_) else {
            return 0;
        };
        track.parse().unwrap_or(0)
    }

    pub fn process_events(&self) -> JoinHandle<()> {
        let mpv = Arc::clone(&self.mpv);
        let mut event_context = EventContext::new(mpv.ctx);
        event_context
            .disable_deprecated_events()
            .expect("failed to disable deprecated events.");
        event_context
            .observe_property("duration", libmpv2::Format::Double, 0)
            .unwrap();
        event_context
            .observe_property("pause", libmpv2::Format::Flag, 1)
            .unwrap();
        event_context
            .observe_property("cache-speed", libmpv2::Format::Int64, 2)
            .unwrap();
        event_context
            .observe_property("track-list", libmpv2::Format::Node, 3)
            .unwrap();
        event_context
            .observe_property("paused-for-cache", libmpv2::Format::Flag, 4)
            .unwrap();
        event_context
            .observe_property("demuxer-cache-time", libmpv2::Format::Int64, 5)
            .unwrap();
        event_context
            .observe_property("time-pos", libmpv2::Format::Int64, 6)
            .unwrap();
        event_context
            .observe_property("volume", libmpv2::Format::Int64, 7)
            .unwrap();
        event_context
            .observe_property("chapter-list", libmpv2::Format::Node, 8)
            .unwrap();
        let event_thread_alive = self.event_thread_alive.clone();
        std::thread::Builder::new()
            .name("mpv event loop".into())
            .spawn(move || {
                loop {
                    let state = event_thread_alive.load(std::sync::atomic::Ordering::SeqCst);
                    match state {
                        SHUTDOWN => break,
                        PAUSED => atomic_wait::wait(&event_thread_alive, PAUSED),
                        _ => {}
                    }

                    match event_context.wait_event(1000.0) {
                        Some(Ok(event)) => match event {
                            Event::PropertyChange { name, change, .. } => match name {
                                "duration" => {
                                    if let PropertyData::Double(dur) = change {
                                        let _ =
                                            MPV_EVENT_CHANNEL.tx.send(ListenEvent::Duration(dur));
                                    }
                                }
                                "pause" => {
                                    if let PropertyData::Flag(pause) = change {
                                        let _ =
                                            MPV_EVENT_CHANNEL.tx.send(ListenEvent::Pause(pause));
                                    }
                                }
                                "cache-speed" => {
                                    if let PropertyData::Int64(speed) = change {
                                        let _ = MPV_EVENT_CHANNEL
                                            .tx
                                            .send(ListenEvent::CacheSpeed(speed));
                                    }
                                }
                                "track-list" => {
                                    if let PropertyData::Node(node) = change {
                                        let _ = MPV_EVENT_CHANNEL
                                            .tx
                                            .send(ListenEvent::TrackList(node_to_tracks(node)));
                                    }
                                }
                                "chapter-list" => {
                                    if let PropertyData::Node(node) = change {
                                        let _ = MPV_EVENT_CHANNEL.tx.send(
                                            ListenEvent::ChapterList(node_to_chapter_list(node)),
                                        );
                                    }
                                }
                                "volume" => {
                                    if let PropertyData::Int64(volume) = change {
                                        let _ =
                                            MPV_EVENT_CHANNEL.tx.send(ListenEvent::Volume(volume));
                                    }
                                }
                                "speed" => {
                                    if let PropertyData::Double(speed) = change {
                                        let _ =
                                            MPV_EVENT_CHANNEL.tx.send(ListenEvent::Speed(speed));
                                    }
                                }
                                "demuxer-cache-time" => {
                                    if let PropertyData::Int64(time) = change {
                                        let _ = MPV_EVENT_CHANNEL
                                            .tx
                                            .send(ListenEvent::DemuxerCacheTime(time));
                                    }
                                }
                                "time-pos" => {
                                    if let PropertyData::Int64(time) = change {
                                        let _ =
                                            MPV_EVENT_CHANNEL.tx.send(ListenEvent::TimePos(time));
                                    }
                                }
                                "paused-for-cache" => {
                                    if let PropertyData::Flag(pause) = change {
                                        let seeking =
                                            mpv.get_property::<bool>("seeking").unwrap_or(false);
                                        let _ = MPV_EVENT_CHANNEL
                                            .tx
                                            .send(ListenEvent::PausedForCache(pause || seeking));
                                    }
                                }
                                _ => {}
                            },
                            Event::Seek { .. } => {
                                let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Seek);
                            }
                            Event::PlaybackRestart { .. } => {
                                let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::PlaybackRestart);
                            }
                            Event::EndFile(r) => {
                                let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Eof(r));
                            }
                            Event::StartFile => {
                                let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::StartFile);
                            }
                            Event::Shutdown => {
                                let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Shutdown);
                            }
                            _ => {}
                        },
                        Some(Err(e)) => {
                            let _ = MPV_EVENT_CHANNEL
                                .tx
                                .send(ListenEvent::Error(e.to_user_facing()));
                        }
                        None => {}
                    };
                }
            })
            .expect("Failed to spawn mpv event loop")
    }
}

unsafe impl Send for TsukimiMPV {}
unsafe impl Sync for TsukimiMPV {}

pub struct MpvTracks {
    pub audio_tracks: Vec<MpvTrack>,
    pub sub_tracks: Vec<MpvTrack>,
}

fn node_to_tracks(node: MpvNode) -> MpvTracks {
    let mut audio_tracks = Vec::new();
    let mut sub_tracks = Vec::new();
    let array = node.array().unwrap();
    for node in array {
        let range = node.map().unwrap().collect::<HashMap<_, _>>();
        let id = range.get("id").unwrap().i64().unwrap();
        let title = range
            .get("title")
            .and_then(|v| v.str())
            .unwrap_or("unknown")
            .to_string();

        let lang = range
            .get("lang")
            .and_then(|v| v.str())
            .unwrap_or("unknown")
            .to_string();

        let type_ = range.get("type").unwrap().str().unwrap().to_string();
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
    }
}

pub struct ChapterList(pub Vec<Chapter>);

impl IntoIterator for ChapterList {
    type Item = Chapter;
    type IntoIter = std::vec::IntoIter<Chapter>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub struct Chapter {
    pub title: String,
    pub time: f64,
}

fn node_to_chapter_list(node: MpvNode) -> ChapterList {
    let mut chapters = Vec::new();
    let array = node.array().unwrap();
    for node in array {
        let range = node.map().unwrap().collect::<HashMap<_, _>>();
        let title = range
            .get("title")
            .and_then(|v| v.str())
            .unwrap_or("unknown")
            .to_string();
        let time = range.get("time").unwrap().f64().unwrap();
        chapters.push(Chapter { title, time });
    }
    ChapterList(chapters)
}

fn get_full_keystr(key: u32, state: gtk::gdk::ModifierType) -> Option<String> {
    let modstr = get_modstr(state);
    let keystr = keyval_to_keystr(key);
    if let Some(keystr) = keystr {
        return Some(format!("{}{}", modstr, keystr));
    }
    None
}

fn get_modstr(state: gtk::gdk::ModifierType) -> String {
    struct ModMap {
        mask: gtk::gdk::ModifierType,
        str: &'static str,
    }

    let mod_map = [
        ModMap {
            mask: gtk::gdk::ModifierType::SHIFT_MASK,
            str: "Shift+",
        },
        ModMap {
            mask: gtk::gdk::ModifierType::CONTROL_MASK,
            str: "Ctrl+",
        },
        ModMap {
            mask: gtk::gdk::ModifierType::ALT_MASK,
            str: "Alt+",
        },
        ModMap {
            mask: gtk::gdk::ModifierType::SUPER_MASK,
            str: "Meta+",
        },
    ];

    let mut result = String::new();

    for mod_item in &mod_map {
        if state.contains(mod_item.mask) {
            result.push_str(mod_item.str);
        }
    }

    result
}

use gtk::glib::translate::FromGlib;
use url::Url;

use super::options_matcher::{
    match_audio_channels,
    match_hwdec_interop,
    match_video_upscale,
};
use crate::{
    client::error::UserFacingError,
    ui::models::SETTINGS,
    utils::spawn_tokio_without_await,
};

const KEYSTRING_MAP: &[(&str, &str)] = &[
    ("PGUP", "Page_Up"),
    ("PGDWN", "Page_Down"),
    ("BS", "\x08"),
    ("SHARP", "#"),
    ("UP", "KP_Up"),
    ("DOWN", "KP_Down"),
    ("RIGHT", "KP_Right"),
    ("LEFT", "KP_Left"),
    ("RIGHT", "Right"),
    ("LEFT", "Left"),
    ("UP", "Up"),
    ("DOWN", "Down"),
    ("ESC", "\x1b"),
    ("DEL", "\x7f"),
    ("ENTER", "\r"),
    ("ENTER", "Return"),
    ("INS", "Insert"),
    ("VOLUME_LOWER", "AudioLowerVolume"),
    ("MUTE", "AudioMute"),
    ("VOLUME_UP", "AudioRaiseVolume"),
    ("PLAY", "AudioPlay"),
    ("STOP", "AudioStop"),
    ("PREV", "AudioPrev"),
    ("NEXT", "AudioNext"),
    ("FORWARD", "AudioForward"),
    ("REWIND", "AudioRewind"),
    ("MENU", "Menu"),
    ("HOMEPAGE", "HomePage"),
    ("MAIL", "Mail"),
    ("FAVORITES", "Favorites"),
    ("SEARCH", "Search"),
    ("SLEEP", "Sleep"),
    ("CANCEL", "Cancel"),
    ("RECORD", "AudioRecord"),
    ("", "Control_L"),
    ("", "Control_R"),
    ("", "Alt_L"),
    ("", "Alt_R"),
    ("", "Meta_L"),
    ("", "Meta_R"),
    ("", "Shift_L"),
    ("", "Shift_R"),
    ("", "grave"),
    ("SPACE", " "),
];

fn keyval_to_keystr(keyval: u32) -> Option<String> {
    let key = unsafe { gtk::gdk::Key::from_glib(keyval) };

    let key_name = if let Some(c) = key.to_unicode() {
        c.to_string()
    } else {
        key.name()?.to_string()
    };

    KEYSTRING_MAP
        .iter()
        .find(|(_, keyval_str)| **keyval_str == key_name)
        .map(|(keystr, _)| keystr.to_string())
        .or(Some(key_name))
}
