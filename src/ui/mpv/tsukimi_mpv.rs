use gtk::gdk::GLContext;
use libmpv2::{
    events::{EventContext, PropertyData},
    GetData, SetData,
};

use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use libmpv2::{
    render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
    Mpv,
};

pub struct TsukimiMPV {
    pub mpv: RefCell<Option<Mpv>>,
    pub ctx: RefCell<Option<RenderContext>>,
    pub event_thread_alive: Arc<AtomicBool>,
}

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
            event_thread_alive: Arc::new(AtomicBool::new(false)),
        }
    }
}

use async_channel::{Receiver, Sender};
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
    let (tx, rx) = async_channel::bounded::<bool>(1);

    RenderUpdate { tx, rx }
});

pub struct SeekingUpdate {
    pub tx: Sender<bool>,
    pub rx: Receiver<bool>,
}

pub static SEEKING_UPDATE: Lazy<SeekingUpdate> = Lazy::new(|| {
    let (tx, rx) = async_channel::bounded::<bool>(1);

    SeekingUpdate { tx, rx }
});

pub struct PauseUpdate {
    pub tx: Sender<bool>,
    pub rx: Receiver<bool>,
}

pub static PAUSE_UPDATE: Lazy<PauseUpdate> = Lazy::new(|| {
    let (tx, rx) = async_channel::bounded::<bool>(1);

    PauseUpdate { tx, rx }
});

pub struct MPVDurationUpdate {
    pub tx: Sender<f64>,
    pub rx: Receiver<f64>,
}

pub static MPV_DURATION_UPDATE: Lazy<MPVDurationUpdate> = Lazy::new(|| {
    let (tx, rx) = async_channel::bounded::<f64>(1);

    MPVDurationUpdate { tx, rx }
});

pub struct MPVEndFile {
    pub tx: Sender<u32>,
    pub rx: Receiver<u32>,
}

pub static MPV_END_FILE: Lazy<MPVEndFile> = Lazy::new(|| {
    let (tx, rx) = async_channel::bounded::<u32>(1);

    MPVEndFile { tx, rx }
});

pub struct MPVError {
    pub tx: Sender<String>,
    pub rx: Receiver<String>,
}

pub static MPV_ERROR: Lazy<MPVError> = Lazy::new(|| {
    let (tx, rx) = async_channel::bounded::<String>(1);

    MPVError { tx, rx }
});

pub struct CacheSpeedUpdate {
    pub tx: Sender<i64>,
    pub rx: Receiver<i64>,
}

pub static CACHE_SPEED_UPDATE: Lazy<CacheSpeedUpdate> = Lazy::new(|| {
    let (tx, rx) = async_channel::bounded::<i64>(1);

    CacheSpeedUpdate { tx, rx }
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
            let _ = RENDER_UPDATE.tx.send_blocking(true);
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
        self.command("loadfile", &[url, "append"]);
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
        let event_thread_alive = Arc::clone(&self.event_thread_alive);
        std::thread::Builder::new()
            .name("mpv event loop".into())
            .spawn(move || {
                while event_thread_alive.load(Ordering::Relaxed) {
                    match event_context.wait_event(0.5) {
                        Some(Ok(event)) => match event {
                            Event::PropertyChange { name, change, .. } => match name {
                                "duration" => {
                                    if let PropertyData::Double(dur) = change {
                                        let _ = MPV_DURATION_UPDATE.tx.send_blocking(dur);
                                    }
                                }
                                "pause" => {
                                    if let PropertyData::Flag(pause) = change {
                                        let _ = PAUSE_UPDATE.tx.send_blocking(pause);
                                    }
                                }
                                "cache-speed" => {
                                    if let PropertyData::Int64(speed) = change {
                                        let _ = CACHE_SPEED_UPDATE.tx.send_blocking(speed);
                                    }
                                }
                                _ => {}
                            },
                            Event::Seek { .. } => {
                                let _ = SEEKING_UPDATE.tx.send_blocking(true);
                            }
                            Event::PlaybackRestart { .. } => {
                                let _ = SEEKING_UPDATE.tx.send_blocking(false);
                            }
                            Event::EndFile(r) => {
                                let _ = MPV_END_FILE.tx.send_blocking(r);
                            }
                            _ => {}
                        },
                        Some(Err(e)) => {
                            let _ = MPV_ERROR.tx.send_blocking(format!("{}", e));
                        }
                        None => {}
                    };
                }
            })
            .expect("Failed to spawn mpv event loop");
    }
}

unsafe impl Send for TsukimiMPV {}
unsafe impl Sync for TsukimiMPV {}
