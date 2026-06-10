use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
use tracing::info;

use crate::video::{
    backend::{TrackKind, TrackSelection},
    mpv::contexted::ContextedMPV,
};

use super::RENDER_UPDATE;

mod imp {
    use crate::video::{MPV_CTRL, MpvMessage, MutsumiMpvError, mpv::contexted::ContextedMPV};
    use libmpv2::Mpv;
    use std::{
        ffi::c_void,
        sync::{Arc, OnceLock},
    };

    use super::*;

    use flume::bounded;
    use gdk_wayland::{WaylandDisplay, wayland_client::Proxy};

    use glib::subclass::Signal;
    use glow::HasContext;
    use gtk::{
        gdk::{Display, GLContext},
        glib,
        prelude::*,
    };
    use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
    use once_cell::sync::OnceCell;

    #[derive(Default)]
    pub struct MPVGLArea {
        pub mpv: ContextedMPV,
        pub mpv_ctx: OnceCell<RenderContext>,
        pub gl_ctx: OnceCell<glow::Context>,
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

            self.obj().set_hexpand(true);
            self.obj().set_vexpand(true);
        }

        fn dispose(&self) {
            self.mpv.shutdown();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("mutsumi-error")
                        .param_types([glib::Type::I32])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for MPVGLArea {
        fn realize(&self) {
            self.parent_realize();

            let obj = self.obj();
            if obj.error().is_some() {
                self.throw_error(MutsumiMpvError::AreaNotInitialized);
                return;
            }

            obj.make_current();
            let Some(gl_context) = obj.context() else {
                self.throw_error(MutsumiMpvError::ContextNotInitialized);
                return;
            };

            self.setup_mpv(gl_context, obj.display());

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
            let Some(ctx) = self.mpv_ctx.get() else {
                return glib::Propagation::Stop;
            };

            let factor = self.obj().scale_factor();
            let width = self.obj().width() * factor;
            let height = self.obj().height() * factor;

            unsafe {
                let fbo = self.glow_cxt().get_parameter_i32(glow::FRAMEBUFFER_BINDING);
                ctx.render::<GLContext>(fbo, width, height, true).ok();
            }
            glib::Propagation::Stop
        }
    }

    impl MPVGLArea {
        fn setup_mpv(&self, gl_context: GLContext, display: Display) {
            let mut render_params = vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address,
                    ctx: gl_context,
                }),
            ];

            // MPV render params to enable hardware decoding on Wayland displays.
            //
            // https://github.com/mpv-player/mpv/blob/86e12929aa0bbc61946d3804982acf887786a7cb/include/mpv/render_gl.h#L91
            if let Some(display_wrapper) = display.downcast::<WaylandDisplay>().ok()
                && let Some(wl_display) = display_wrapper.wl_display()
            {
                render_params.push(RenderParam::WaylandDisplay(
                    wl_display.id().as_ptr() as *const c_void
                ));
            }

            let (arc_tx, arc_rx) = bounded::<Arc<Mpv>>(1);

            MPV_CTRL
                .tx
                .send(MpvMessage::InitRenderContext(arc_tx))
                .expect("Init render context failed");

            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    let mpv = arc_rx.recv_async().await.expect("Actor dropped sender");
                    let mut handle = mpv.ctx;
                    let mut ctx = RenderContext::new(unsafe { handle.as_mut() }, render_params)
                        .expect("Failed creating render context");
                    ctx.set_update_callback(|| {
                        let _ = RENDER_UPDATE.tx.send(true);
                    });
                    imp.mpv_ctx
                        .set(ctx)
                        .ok()
                        .expect("MPV render context already set???");
                }
            ));
        }

        fn glow_cxt(&self) -> &glow::Context {
            self.gl_ctx.get_or_init(|| unsafe {
                glow::Context::from_loader_function(epoxy::get_proc_addr)
            })
        }

        fn throw_error(&self, code: MutsumiMpvError) {
            self.obj().emit_by_name::<()>("mutsumi-error", &[&code]);
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

    pub fn mpv(&self) -> &ContextedMPV {
        &self.imp().mpv
    }

    pub fn play(&self, url: &str, start_seconds: f64) {
        let url = url.to_owned();

        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let mpv = obj.mpv();

                info!("Now Playing: {}", url);
                mpv.load_video(&url);

                mpv.set_start(start_seconds);
                mpv.pause(false);
            }
        ));
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.mpv().press_key(key, state)
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.mpv().release_key(key, state)
    }

    pub fn volume_scroll(&self, value: i64) {
        self.mpv().volume_scroll(value)
    }

    pub fn set_slang(&self, value: String) {
        self.mpv().set_slang(value)
    }

    pub fn shutdown(&self) {
        self.mpv().shutdown();
    }

    pub fn stop(&self) {
        self.mpv().stop();
    }

    pub fn load_video(&self, url: &str) {
        self.mpv().load_video(url);
    }

    pub fn pause(&self, pause: bool) {
        self.mpv().pause(pause);
    }

    pub fn command_pause(&self) {
        self.mpv().command_pause();
    }

    pub fn set_percent_position(&self, value: f64) {
        self.mpv().set_percent_position(value);
    }

    pub fn set_start(&self, start_seconds: f64) {
        self.mpv().set_start(start_seconds);
    }

    pub fn set_aid(&self, value: TrackSelection) {
        self.mpv().set_aid(value);
    }

    pub fn set_sid(&self, value: TrackSelection) {
        self.mpv().set_sid(value);
    }

    pub fn disable_aid(&self) {
        self.mpv().set_aid(TrackSelection::None);
    }

    pub fn disable_sid(&self) {
        self.mpv().set_sid(TrackSelection::None);
    }

    pub fn set_brightness(&self, value: f64) {
        self.mpv().mpv.set_property("brightness", value);
    }

    pub fn set_contrast(&self, value: f64) {
        self.mpv().mpv.set_property("contrast", value);
    }

    pub fn set_gamma(&self, value: f64) {
        self.mpv().mpv.set_property("gamma", value);
    }

    pub fn set_hue(&self, value: f64) {
        self.mpv().mpv.set_property("hue", value);
    }

    pub fn set_saturation(&self, value: f64) {
        self.mpv().mpv.set_property("saturation", value);
    }

    pub fn set_sub_pos(&self, value: f64) {
        self.mpv().mpv.set_property("sub-pos", value);
    }

    pub fn set_sub_font_size(&self, value: f64) {
        self.mpv().mpv.set_property("sub-font-size", value);
    }

    pub fn set_sub_scale(&self, value: f64) {
        self.mpv().mpv.set_property("sub-scale", value);
    }

    pub fn set_sub_speed(&self, value: f64) {
        self.mpv().mpv.set_property("sub-speed", value);
    }

    pub fn set_sub_delay(&self, value: f64) {
        self.mpv().mpv.set_property("sub-delay", value);
    }

    pub fn set_sub_bold(&self, value: bool) {
        self.mpv().mpv.set_property("sub-bold", value);
    }

    pub fn set_sub_italic(&self, value: bool) {
        self.mpv().mpv.set_property("sub-italic", value);
    }

    pub fn set_sub_font(&self, value: &str) {
        self.mpv().mpv.set_property("sub-font", value.to_owned());
    }

    pub fn set_sub_color(&self, value: &str) {
        self.mpv().mpv.set_property("sub-color", value.to_owned());
    }

    pub fn set_sub_border_color(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("sub-border-color", value.to_owned());
    }

    pub fn set_sub_back_color(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("sub-back-color", value.to_owned());
    }

    pub fn set_sub_border_style(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("sub-border-style", value.to_owned());
    }

    pub fn set_sub_border_size(&self, value: f64) {
        self.mpv().mpv.set_property("sub-border-size", value);
    }

    pub fn set_sub_shadow_offset(&self, value: f64) {
        self.mpv().mpv.set_property("sub-shadow-offset", value);
    }

    pub fn set_audio_delay(&self, value: f64) {
        self.mpv().mpv.set_property("audio-delay", value);
    }

    pub fn set_audio_channels(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("audio-channels", value.to_owned());
    }

    pub fn set_audio_pan(&self, value: &str) {
        self.mpv().mpv.set_property("af", value.to_owned());
    }

    pub fn clear_audio_pan(&self) {
        self.mpv().mpv.set_property("af", String::new());
    }

    pub fn set_scale(&self, value: &str) {
        self.mpv().mpv.set_property("scale", value.to_owned());
    }

    pub fn set_deband(&self, value: bool) {
        self.mpv().mpv.set_property("deband", value);
    }

    pub fn set_deband_iterations(&self, value: i64) {
        self.mpv().mpv.set_property("deband-iterations", value);
    }

    pub fn set_deband_threshold(&self, value: i64) {
        self.mpv().mpv.set_property("deband-threshold", value);
    }

    pub fn set_deband_range(&self, value: i64) {
        self.mpv().mpv.set_property("deband-range", value);
    }

    pub fn set_deband_grain(&self, value: i64) {
        self.mpv().mpv.set_property("deband-grain", value);
    }

    pub fn set_deinterlace(&self, value: bool) {
        self.mpv().mpv.set_property("deinterlace", value);
    }

    pub fn set_hwdec(&self, value: &str) {
        self.mpv().mpv.set_property("hwdec", value.to_owned());
    }

    pub fn set_panscan(&self, value: f64) {
        self.mpv().mpv.set_property("panscan", value);
    }

    pub fn set_stretch_image_subs_to_screen(&self, value: bool) {
        self.mpv()
            .mpv
            .set_property("stretch-image-subs-to-screen", value);
    }

    pub fn set_demuxer_max_bytes(&self, value: &str) {
        self.mpv()
            .mpv
            .set_property("demuxer-max-bytes", value.to_owned());
    }

    pub fn set_cache_secs(&self, value: f64) {
        self.mpv().mpv.set_property("cache-secs", value);
    }

    pub fn display_stats_toggle(&self) {
        self.mpv().display_stats_toggle();
    }

    pub fn add_sub(&self, url: &str) {
        self.mpv().add_sub(url);
    }

    pub fn set_position(&self, position: f64) {
        self.mpv().set_position(position);
    }

    pub fn set_volume(&self, volume: i64) {
        self.mpv().set_volume(volume);
    }

    pub fn seek_forward(&self, seconds: i64) {
        self.mpv().seek_forward(seconds);
    }

    pub fn seek_backward(&self, seconds: i64) {
        self.mpv().seek_backward(seconds);
    }

    pub fn set_speed(&self, speed: f64) {
        self.mpv().set_speed(speed);
    }

    pub fn set_keep_aspect_ratio(&self, value: bool) {
        self.mpv().mpv.set_property("keepaspect", value);
    }

    pub async fn position(&self) -> f64 {
        self.mpv().position().await
    }

    pub async fn paused(&self) -> bool {
        self.mpv().paused().await
    }

    pub async fn duration(&self) -> f64 {
        self.mpv().duration().await
    }

    pub async fn get_track_id(&self, kind: TrackKind) -> i64 {
        let type_ = match kind {
            TrackKind::Video => "vid",
            TrackKind::Audio => "aid",
            TrackKind::Subtitle => "sid",
        };

        self.mpv().get_track_id(type_).await
    }
}
