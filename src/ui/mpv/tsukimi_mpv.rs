use gtk::gdk::GLContext;
use libmpv2::{GetData, SetData};

use std::cell::RefCell;

use libmpv2::{
    render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
    Mpv,
};

pub struct TsukimiMPV {
    pub mpv: RefCell<Option<Mpv>>,
    pub ctx: RefCell<Option<RenderContext>>,
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
            init.set_property("osc", true)?;
            init.set_property("config", true)?;
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
        }
    }
}

use async_channel::{Receiver, Sender};
use libc::c_void;
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
        self.set_property("percent-pos", value);
    }

    pub fn position(&self) -> f64 {
        self.get_property("percent-pos").unwrap_or(0.0)
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
        'event: loop {
            match mpv.event_context_mut().wait_event(0.0) {
                Some(Ok(event)) => {
                    self.handle_event(&event);
                }
                Some(Err(e)) => break 'event,
                None => break 'event,
            }
        }
    }

    fn handle_event(&self, event: &libmpv2::events::Event) {
        match event {
            _ => {
                println!("Event: {:?}", event);
            }
        }
    }
}

unsafe impl Send for TsukimiMPV {}
unsafe impl Sync for TsukimiMPV {}
