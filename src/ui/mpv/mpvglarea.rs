use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::client::client::EMBY_CLIENT;
use crate::client::structs::Back;
use crate::ui::models::SETTINGS;

mod imp {

    use std::sync::Mutex;
    use gtk::gdk::GLContext;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::glib;
    use libc::c_void;
    use libmpv2::{
        render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
        Mpv,
    };

    fn get_proc_address(_ctx: &GLContext, name: &str)  -> *mut c_void  {
        epoxy::get_proc_addr(name) as *mut c_void
    }

    // Object holding the state
    #[derive(Default)]
    pub struct MPVGLArea {
        pub mpv: Mutex<Option<Mpv>>,
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
            
            self.mpv.lock().unwrap().replace(mpv);

            self.obj().set_has_stencil_buffer(true);
            
            self.obj().add_tick_callback(|area, _| {
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
            let mut binding = self.mpv.lock().unwrap();
            let mpv = binding.as_mut().unwrap();
            let ctx  = RenderContext::new(
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

            self.ctx.lock().unwrap().replace(ctx);
        }

        fn unrealize(&self) {
            self.parent_unrealize();
            if let Some(mpv) = self.mpv.lock().unwrap().take() {
                drop(mpv);
            }
            if let Some(ctx) = self.ctx.lock().unwrap().take() {
                drop(ctx);
            }
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
        let bind = self.imp().mpv.lock().unwrap();
        let Some(mpv) = bind.as_ref() else {
            return;
        };
        let url = EMBY_CLIENT.get_streaming_url(url);
        mpv.command("loadfile", &[&url, "replace"]).unwrap();

        if let Some(suburi) = suburi {
            let suburl = EMBY_CLIENT.get_streaming_url(suburi);
            mpv.command("sub-add", &[&suburl]).unwrap();
        }

        mpv.set_property("start", format!("{}%", percentage as u32)).unwrap();

    }

    pub fn set_position(&self, value: f64) {
        let bind = self.imp().mpv.lock().unwrap();
        let Some(mpv) = bind.as_ref() else {
            return;
        };
        mpv.set_property("percent-pos", value).unwrap();
    }

    pub fn position(&self) -> f64 {
        let bind = self.imp().mpv.lock().unwrap();
        let Some(mpv) = bind.as_ref() else {
            return 0.0;
        };
        mpv.get_property("percent-pos").unwrap()
    }
}
