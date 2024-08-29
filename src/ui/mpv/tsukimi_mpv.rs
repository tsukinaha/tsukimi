use gtk::gdk::GLContext;
use libmpv2::{
    events::{EventContext, PropertyData}, GetData, SetData
};
use tokio::time;

use std::{
    cell::RefCell,
    sync::{
        atomic::AtomicU32,
        Arc,
    },
};

use libmpv2::{
    render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
    Mpv,
};

pub struct MpvTrack {
    pub id: i64,
    pub title: String,
}

pub struct TsukimiMPV {
    pub mpv: RefCell<Option<Mpv>>,
    pub ctx: RefCell<Option<RenderContext>>,
    pub event_thread_alive: Arc<AtomicU32>,
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
            init.set_property("config", false)?;
            init.set_property("input-vo-keyboard", true)?;
            init.set_property("input-default-bindings", true)?;
            init.set_property("user-agent", "Tsukimi")?;
            init.set_property("vo", "libmpv")?;
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

use flume::{unbounded, Sender, Receiver};
use libc::c_void;
use libmpv2::events::Event;
use once_cell::sync::Lazy;

fn get_proc_address(_ctx: &GLContext, name: &str) -> *mut c_void {
    epoxy::get_proc_addr(name) as *mut c_void
}

pub struct RenderUpdate {
    pub tx: Sender<bool>,
    pub rx: Receiver<bool>,
}

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

    pub fn stop(&self) {
        self.command("stop", &[]);
    }

    pub fn display_stats_toggle(&self) {
        self.command("script-binding", &["stats/display-stats-toggle"]);
    }

    pub fn get_audio_and_subtitle_tracks(&self) -> (Vec<MpvTrack>, Vec<MpvTrack>) {
        let tracks = self.get_property("track-list/count").unwrap_or(0);
        let mut audio_tracks = Vec::new();
        let mut sub_tracks = Vec::new();
        for i in 0..tracks {
            let track_type = self.get_property(&format!("track-list/{}", i)).unwrap_or("Unknown".to_string());
            if track_type == "audio" {
                let audio_track_title = self.get_property(&format!("track-list/{}/title", i)).unwrap_or("Unknown".to_string());
                let audio_track_id = self.get_property(&format!("track-list/{}/id", i)).unwrap_or(0);
                audio_tracks.push(MpvTrack {
                    id: audio_track_id,
                    title: audio_track_title,
                });
            } else if track_type == "sub" {
                let sub_track_title = self.get_property(&format!("track-list/{}/title", i)).unwrap_or("Unknown".to_string());
                let sub_track_id = self.get_property(&format!("track-list/{}/id", i)).unwrap_or(0);
                sub_tracks.push(MpvTrack {
                    id: sub_track_id,
                    title: sub_track_title,
                });
            }
        }
        (audio_tracks, sub_tracks)
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
        let event_thread_alive = self.event_thread_alive.clone();
        std::thread::Builder::new()
            .name("mpv event loop".into())
            .spawn(move || {
                loop {
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
                                        let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::CacheSpeed(speed));
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
                        None => {
                        }
                    };
                    std::thread::sleep(time::Duration::from_millis(50));
                }
            })
            .expect("Failed to spawn mpv event loop");
    }
}

unsafe impl Send for TsukimiMPV {}
unsafe impl Sync for TsukimiMPV {}
