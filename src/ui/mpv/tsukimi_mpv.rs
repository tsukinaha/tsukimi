use gtk::gdk::GLContext;
use libmpv2::{
    events::{EventContext, PropertyData},
    mpv_node::MpvNode,
    GetData, SetData,
};
use tokio::time;

use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc},
};

use libmpv2::{
    render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
    Mpv,
};

#[derive(Debug)]
pub struct MpvTrack {
    pub id: i64,
    pub title: String,
    pub lang: String,
    pub type_: String,
}

pub struct TsukimiMPV {
    pub mpv: RefCell<Option<Mpv>>,
    pub ctx: RefCell<Option<RenderContext>>,
    pub event_thread_alive: Arc<AtomicU32>,
}

pub enum TrackSelection {
    Track(i64),
    None,
}

impl TrackSelection {
    pub fn to_string(&self) -> String {
        match self {
            TrackSelection::Track(id) => id.to_string(),
            TrackSelection::None => "no".to_string(),
        }
    }
}

pub const PAUSED: u32 = 0;
pub const ACTIVE: u32 = 1;

impl Default for TsukimiMPV {
    fn default() -> Self {
        unsafe {
            use libc::setlocale;
            use libc::LC_NUMERIC;
            setlocale(LC_NUMERIC, "C\0".as_ptr() as *const _);
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
            init.set_property("input-vo-keyboard", true)?;
            init.set_property("input-default-bindings", true)?;
            init.set_property("user-agent", "Tsukimi")?;
            if SETTINGS.mpv() {
                init.set_property("vo", "gpu-next")?;
            } else {
                init.set_property("vo", "libmpv")?;
            }
            if SETTINGS.mpv_estimate() {
                let fps = SETTINGS.mpv_estimate_target_frame();
                init.set_property("vf", format!("lavfi=\"fps=fps={fps}:round=down\""))?;
            }
            Ok(())
        })
        .unwrap();

        Self {
            mpv: RefCell::new(Some(mpv)),
            ctx: RefCell::new(None),
            event_thread_alive: Arc::new(AtomicU32::new(PAUSED)),
        }
    }
}

use flume::{unbounded, Receiver, Sender};
use libc::c_void;
use libmpv2::events::Event;
use once_cell::sync::Lazy;

use crate::ui::models::SETTINGS;

fn get_proc_address(_ctx: &GLContext, name: &str) -> *mut c_void {
    epoxy::get_proc_addr(name) as *mut c_void
}

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
}

pub static MPV_EVENT_CHANNEL: Lazy<MPVEventChannel> = Lazy::new(|| {
    let (tx, rx) = unbounded::<ListenEvent>();

    MPVEventChannel { tx, rx }
});

impl TsukimiMPV {
    pub fn connect_render_update(&self, gl_context: GLContext) {
        let mut binding = self.mpv.borrow_mut();
        let mpv = binding.as_mut().unwrap();
        let mut ctx = RenderContext::new(
            unsafe { mpv.ctx.as_mut() },
            vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address,
                    ctx: gl_context,
                }),
            ],
        )
        .expect("Failed creating render context");

        ctx.set_update_callback(|| {
            let _ = RENDER_UPDATE.tx.send(true);
        });

        self.ctx.replace(Some(ctx));
    }

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

    pub fn set_speed(&self, speed: f64) {
        self.set_property("speed", speed);
    }

    pub fn set_aid(&self, aid: TrackSelection) {
        self.set_property("aid", aid.to_string());
    }

    pub fn set_sid(&self, sid: TrackSelection) {
        self.set_property("sid", sid.to_string());
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.command("keypress", &[&get_full_keystr(key, state)]);
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.command("keyup", &[&get_full_keystr(key, state)]);
    }

    pub fn stop(&self) {
        self.command("stop", &[]);
    }

    pub fn display_stats_toggle(&self) {
        self.command("script-binding", &["stats/display-stats-toggle"]);
    }

    fn set_property<V>(&self, property: &str, value: V)
    where
        V: SetData,
    {
        let bind = self.mpv.borrow();
        let Some(mpv) = bind.as_ref() else {
            return;
        };
        mpv.set_property(property, value).unwrap();
    }

    fn get_property<V>(&self, property: &str) -> Option<V>
    where
        V: GetData,
    {
        let bind = self.mpv.borrow();
        let mpv = bind.as_ref()?;
        mpv.get_property(property).ok()
    }

    fn command(&self, cmd: &str, args: &[&str]) {
        let bind = self.mpv.borrow();
        let Some(mpv) = bind.as_ref() else {
            return;
        };
        mpv.command(cmd, args).unwrap();
    }

    pub fn get_track_id(&self, type_: &str) -> i64 {
        let Some(track) = self.get_property::<String>(&type_) else {
            return 0;
        };
        track.parse().unwrap_or(0)
    }

    pub fn process_events(&self) {
        let mut bind = self.mpv.borrow_mut();
        let Some(mpv) = bind.as_mut() else {
            return;
        };
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
        let event_thread_alive = self.event_thread_alive.clone();
        std::thread::Builder::new()
            .name("mpv event loop".into())
            .spawn(move || loop {
                atomic_wait::wait(&event_thread_alive, PAUSED);
                match event_context.wait_event(1000.0) {
                    Some(Ok(event)) => match event {
                        Event::PropertyChange { name, change, .. } => match name {
                            "duration" => {
                                if let PropertyData::Double(dur) = change {
                                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Duration(dur));
                                }
                            }
                            "pause" => {
                                if let PropertyData::Flag(pause) = change {
                                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Pause(pause));
                                }
                            }
                            "cache-speed" => {
                                if let PropertyData::Int64(speed) = change {
                                    let _ =
                                        MPV_EVENT_CHANNEL.tx.send(ListenEvent::CacheSpeed(speed));
                                }
                            }
                            "track-list" => {
                                if let PropertyData::Node(node) = change {
                                    let _ = MPV_EVENT_CHANNEL
                                        .tx
                                        .send(ListenEvent::TrackList(node_to_tracks(node)));
                                }
                            }
                            "volume" => {
                                if let PropertyData::Int64(volume) = change {
                                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Volume(volume));
                                }
                            }
                            "speed" => {
                                if let PropertyData::Double(speed) = change {
                                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Speed(speed));
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
                        _ => {}
                    },
                    Some(Err(e)) => {
                        let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Error(e.to_string()));
                    }
                    None => {}
                };
                std::thread::sleep(time::Duration::from_millis(50));
            })
            .expect("Failed to spawn mpv event loop");
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

fn get_full_keystr(key: u32, state: gtk::gdk::ModifierType) -> String {
    let modstr = get_modstr(state);
    let keystr = keyval_to_keystr(key);
    format!("{}{}", modstr, keystr)
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

fn keyval_to_keystr(keyval: u32) -> String {
    let key = unsafe { gtk::gdk::Key::from_glib(keyval) };
    const KEYSTRING_MAP: &[(&str, &str)] = &[];

    if let Some(unicode_char) = char::from_u32(keyval) {
        return unicode_char.to_string();
    }

    if let Some(key_name) = key.name() {
        return key_name.to_string();
    }

    for &(key, key_str) in KEYSTRING_MAP {
        if key_str.eq_ignore_ascii_case(&keyval.to_string()) {
            return key.to_string();
        }
    }

    String::new()
}
