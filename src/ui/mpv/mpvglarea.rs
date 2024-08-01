use std::sync::{Arc, Mutex};

use glib::Object;
use gtk::{gio, glib};
use libmpv2::Mpv;
use once_cell::sync::Lazy;
use gtk::gdk::GLContext;
use libc::c_void;
use crate::client::client::EMBY_CLIENT;
use crate::client::structs::Back;

mod imp {

    use std::sync::Mutex;
    use gtk::gdk::GLContext;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::glib;
    
    use libmpv2::{
        render::{RenderContext},
    };

    

    use super::{EmbyPlayer, MPV};

    // Object holding the state
    #[derive(Default)]
    pub struct MPVGLArea {
        pub ctx: Mutex<Option<RenderContext>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MPVGLArea {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MPVGLArea";
        type Type = super::MPVGLArea;
        type ParentType = gtk::GLArea;
    }

    impl ObjectImpl for MPVGLArea {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.set_has_stencil_buffer(true);
            
            obj.add_tick_callback(|area, _| {
                area.queue_render();
                glib::ControlFlow::Continue
            });
        }
    }

    impl WidgetImpl for MPVGLArea {
        fn realize(&self) {
            self.parent_realize();
            let obj = self.obj();
            obj.make_current();
            let gl_context = self.obj().context().unwrap();
            let ctx = MPV.lock().unwrap().ctx(gl_context);
            self.ctx.lock().unwrap().replace(ctx);
        }
    }

    impl GLAreaImpl for MPVGLArea {
        fn render(&self, _context: &GLContext) -> glib::Propagation {
            if let Some(ctx) = self.ctx.lock().unwrap().as_ref() {
                let factor = self.obj().scale_factor();
                let width = self.obj().width() * factor;
                let height = self.obj().height() * factor;
                unsafe { 
                    let mut fbo = 0;
                    gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fbo);
                    ctx.render::<GLContext>(fbo, width, height, true).unwrap();
                }
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Stop
            }
        }
    }
    
}

glib::wrapper! {
    pub struct MPVGLArea(ObjectSubclass<imp::MPVGLArea>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,gtk::GLArea,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for MPVGLArea {
    fn default() -> Self {
        Self::new()
    }
}

impl MPVGLArea {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn play(&self,
        url: &str,
        suburi: Option<&str>,
        name: Option<&str>,
        line2: Option<&str>,
        back: Option<Back>,
        percentage: f64
    ) {
        let mpv = MPV.lock().unwrap();

        let url = EMBY_CLIENT.get_streaming_url(url);
        mpv.command("loadfile", &[&url, "replace"]).unwrap();

        if let Some(suburi) = suburi {
            let suburl = EMBY_CLIENT.get_streaming_url(suburi);
            mpv.command("sub-add", &[&suburl]).unwrap();
        }

        mpv.set_property("start", format!("{}%", percentage as u32)).unwrap();
    }

    pub fn set_position(&self, value: f64) {
        MPV.lock().unwrap().set_position(value)
    }

    pub fn position(&self) -> f64 {
        MPV.lock().unwrap().position()
    }
}

use libmpv2::{
    render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
};

pub static MPV: Lazy<Arc<Mutex<Mpv>>> = Lazy::new(|| {

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

    gl::load_with(|name|{
        epoxy::get_proc_addr(name) as *const _
    });

    let mpv = Mpv::with_initializer(|init| {
        init.set_property("osc", true)?;
        init.set_property("config", true)?;
        init.set_property("input-vo-keyboard", true)?;
        init.set_property("input-default-bindings", true)?;
        init.set_property("user-agent", "Tsukimi")?;
        init.set_property("vo", "libmpv")?;
        Ok(())
    }).unwrap();

    Arc::new(Mutex::new(mpv))
});

pub trait EmbyPlayer {
    fn listen_events(&mut self);
    fn position(&mut self) -> f64;
    fn set_position(&mut self, value: f64);
    fn ctx(&mut self, glcontext: GLContext) -> RenderContext;
    fn paused(&mut self) -> bool;
    fn pause(&mut self, value: bool);
    fn quit(&mut self);
}

fn get_proc_address(_ctx: &GLContext, name: &str)  -> *mut c_void  {
    epoxy::get_proc_addr(name) as *mut c_void
}

impl EmbyPlayer for Mpv {
    fn listen_events(&mut self) {
        let ev_ctx = self.event_context_mut();
        crossbeam::scope(|scope| {
            scope.spawn(move |_| loop {
                let ev = ev_ctx.wait_event(1.);
                match ev {
                    Some(Ok(libmpv2::events::Event::EndFile(r))) => {
                        println!("End of file: {:?}", r);
                    }
                    Some(Ok(mpv_event)) => {
                        eprintln!("MPV event: {:?}", mpv_event);
                    }
                    Some(Err(err)) => {
                        eprintln!("MPV Error: {}", err);
                    }
                    None => {}
                }
            });
        }).unwrap();
    }

    fn position(&mut self) -> f64 {
        self.get_property("percent-pos").unwrap()
    }

    fn set_position(&mut self, value: f64) {
        self.set_property("percent-pos", value).unwrap();
    }

    fn ctx(&mut self, glcontext: GLContext) -> RenderContext {
        RenderContext::new(
            unsafe { self.ctx.as_mut() },
            vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address,
                    ctx: glcontext,
                }),
            ],
        )
        .expect("Failed creating render context")
    }

    fn paused(&mut self) -> bool {
        self.get_property("pause").unwrap()
    }

    fn pause(&mut self, value: bool) {
        self.set_property("pause", value).unwrap();
    }

    fn quit(&mut self) {
        self.command("quit", &[]).unwrap();
    }
}