use glib::Object;
use gtk::{
    gio,
    glib,
    subclass::prelude::*,
};
use libmpv2::SetData;
use tracing::info;

use super::tsukimi_mpv::{
    ACTIVE,
    TrackSelection,
};
use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    utils::spawn,
};

mod imp {
    use std::ffi::c_void;

    use gettextrs::gettext;
    use glow::HasContext;
    use gtk::{
        gdk::GLContext,
        glib,
        prelude::*,
        subclass::prelude::*,
    };
    use libmpv2::render::{
        OpenGLInitParams,
        RenderContext,
        RenderParam,
        RenderParamApiType,
    };
    use once_cell::sync::OnceCell;

    use crate::{
        close_on_error,
        ui::mpv::tsukimi_mpv::{
            RENDER_UPDATE,
            TsukimiMPV,
        },
    };

    #[derive(Default)]
    pub struct MPVGLArea {
        pub mpv: TsukimiMPV,

        pub ctx: OnceCell<glow::Context>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MPVGLArea {
        const NAME: &'static str = "MPVGLArea";
        type Type = super::MPVGLArea;
        type ParentType = gtk::GLArea;
    }

    impl ObjectImpl for MPVGLArea {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            self.mpv().shutdown_event_thread();
        }
    }

    impl WidgetImpl for MPVGLArea {
        fn realize(&self) {
            self.parent_realize();
            let obj = self.obj();

            if obj.error().is_some() {
                close_on_error!(obj, gettext("Failed to realize GLArea"));
                return;
            }

            obj.make_current();
            let Some(gl_context) = obj.context() else {
                close_on_error!(obj, gettext("Failed to get GLContext"));
                return;
            };

            self.setup_mpv(gl_context);

            glib::spawn_future_local(glib::clone!(
                #[weak]
                obj,
                async move {
                    while RENDER_UPDATE.rx.recv_async().await.is_ok() {
                        obj.queue_render();
                    }
                }
            ));
        }

        fn unrealize(&self) {
            self.parent_unrealize();
        }
    }

    impl GLAreaImpl for MPVGLArea {
        fn render(&self, _context: &GLContext) -> glib::Propagation {
            let binding = self.mpv().ctx.borrow();
            let Some(ctx) = binding.as_ref() else {
                return glib::Propagation::Stop;
            };

            let factor = self.obj().scale_factor();
            let width = self.obj().width() * factor;
            let height = self.obj().height() * factor;

            unsafe {
                let fbo = self.glow_cxt().get_parameter_i32(glow::FRAMEBUFFER_BINDING);
                ctx.render::<GLContext>(fbo, width, height, true).unwrap();
            }
            glib::Propagation::Stop
        }
    }

    impl MPVGLArea {
        pub fn mpv(&self) -> &TsukimiMPV {
            &self.mpv
        }

        fn setup_mpv(&self, gl_context: GLContext) {
            let tmpv = self.mpv();
            let mut handle = tmpv.mpv.ctx;
            let mut ctx = RenderContext::new(
                unsafe { handle.as_mut() },
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

            tmpv.ctx.replace(Some(ctx));

            tmpv.process_events();
        }

        fn glow_cxt(&self) -> &glow::Context {
            self.ctx.get_or_init(|| unsafe {
                glow::Context::from_loader_function(epoxy::get_proc_addr)
            })
        }
    }

    fn get_proc_address(_ctx: &GLContext, name: &str) -> *mut c_void {
        epoxy::get_proc_addr(name) as *mut c_void
    }
}

glib::wrapper! {
    pub struct MPVGLArea(ObjectSubclass<imp::MPVGLArea>)
        @extends gtk::Widget ,gtk::GLArea,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::ShortcutManager;
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

    pub fn play(&self, url: &str, percentage: f64) {
        let url = url.to_owned();

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let mpv = &obj.imp().mpv();

                mpv.event_thread_alive
                    .store(ACTIVE, std::sync::atomic::Ordering::SeqCst);
                atomic_wait::wake_all(&*mpv.event_thread_alive);

                let url = JELLYFIN_CLIENT.get_streaming_url(&url).await;

                info!("Now Playing: {}", url);
                mpv.load_video(&url);

                mpv.set_start(percentage);

                mpv.pause(false);
            }
        ));
    }

    pub fn add_sub(&self, url: &str) {
        self.imp().mpv().add_sub(url)
    }

    pub fn seek_forward(&self, value: i64) {
        self.imp().mpv().seek_forward(value)
    }

    pub fn seek_backward(&self, value: i64) {
        self.imp().mpv().seek_backward(value)
    }

    pub fn set_position(&self, value: f64) {
        self.imp().mpv().set_position(value)
    }

    pub fn position(&self) -> f64 {
        self.imp().mpv().position()
    }

    pub fn set_aid(&self, value: TrackSelection) {
        self.imp().mpv().set_aid(value)
    }

    pub fn get_track_id(&self, type_: &str) -> i64 {
        self.imp().mpv().get_track_id(type_)
    }

    pub fn set_sid(&self, value: TrackSelection) {
        self.imp().mpv().set_sid(value)
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.imp().mpv().press_key(key, state)
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.imp().mpv().release_key(key, state)
    }

    pub fn set_speed(&self, value: f64) {
        self.imp().mpv().set_speed(value)
    }

    pub fn set_volume(&self, value: i64) {
        self.imp().mpv().set_volume(value)
    }

    pub fn display_stats_toggle(&self) {
        self.imp().mpv().display_stats_toggle()
    }

    pub fn paused(&self) -> bool {
        self.imp().mpv().paused()
    }

    pub fn pause(&self) {
        self.imp().mpv().command_pause();
    }

    pub fn volume_scroll(&self, value: i64) {
        self.imp().mpv().volume_scroll(value)
    }

    pub fn set_slang(&self, value: String) {
        self.imp().mpv().set_slang(value)
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: SetData + Send + 'static,
    {
        self.imp().mpv().set_property(property, value)
    }
}
