use std::{cell::OnceCell, rc::Rc};

use glib::Object;
use gtk::{gdk::ModifierType, glib, prelude::*, subclass::prelude::*};

use super::{
    GstVideo, GstVideoError, MPVGLArea,
    backend::{BoxedFuture, TrackKind, TrackSelection, VideoBackend},
};

#[derive(Clone, glib::Boxed)]
#[boxed_type(name = "MutsumiVideoBackendHandle")]
pub struct BackendHandle(pub Rc<dyn VideoBackend>);

#[derive(Debug)]
pub enum VideoPlayerNewError {
    InvalidBackend(String),
    Gst(GstVideoError),
}

impl From<GstVideoError> for VideoPlayerNewError {
    fn from(value: GstVideoError) -> Self {
        Self::Gst(value)
    }
}

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::MutsumiVideoPlayer)]
    pub struct MutsumiVideoPlayer {
        #[property(get, set, construct_only)]
        pub backend_name: OnceCell<String>,

        pub backend: OnceCell<BackendHandle>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MutsumiVideoPlayer {
        const NAME: &'static str = "MutsumiVideoPlayer";
        type Type = super::MutsumiVideoPlayer;
        type ParentType = gtk::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MutsumiVideoPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_orientation(gtk::Orientation::Vertical);
            obj.set_spacing(0);
            obj.set_hexpand(true);
            obj.set_vexpand(true);

            match obj.backend_name().as_str() {
                "mpvgl" => {
                    let player = MPVGLArea::new();
                    obj.append(&player);
                    let _ = self.backend.set(BackendHandle(Rc::new(player)));
                }
                "gst" => {
                    let player = GstVideo::new().expect("Failed to create GstVideo");
                    obj.append(&player);
                    let _ = self.backend.set(BackendHandle(Rc::new(player)));
                }
                _ => panic!("Invalid backend name: {}", obj.backend_name()),
            }
        }
    }

    impl WidgetImpl for MutsumiVideoPlayer {}
    impl BoxImpl for MutsumiVideoPlayer {}
}

glib::wrapper! {
    pub struct MutsumiVideoPlayer(ObjectSubclass<imp::MutsumiVideoPlayer>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl MutsumiVideoPlayer {
    pub fn new(backend: &str) -> Self {
        Object::builder().property("backend-name", backend).build()
    }

    pub fn backend_handle(&self) -> &BackendHandle {
        self.imp()
            .backend
            .get()
            .expect("VideoPlayer backend must be initialized during construction")
    }

    fn backend_ref(&self) -> &dyn VideoBackend {
        self.backend_handle().0.as_ref()
    }

    fn backend_rc(&self) -> Rc<dyn VideoBackend> {
        self.backend_handle().0.clone()
    }
}

impl VideoBackend for MutsumiVideoPlayer {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "VideoPlayer"
    }

    fn play(&self, url: &str, percentage: f64) {
        self.backend_ref().play(url, percentage);
    }

    fn shutdown(&self) {
        self.backend_ref().shutdown();
    }

    fn stop(&self) {
        self.backend_ref().stop();
    }

    fn load_video(&self, url: &str) {
        self.backend_ref().load_video(url);
    }

    fn add_sub(&self, url: &str) {
        self.backend_ref().add_sub(url);
    }

    fn pause(&self, pause: bool) {
        self.backend_ref().pause(pause);
    }

    fn command_pause(&self) {
        self.backend_ref().command_pause();
    }

    fn set_position(&self, value: f64) {
        self.backend_ref().set_position(value);
    }

    fn set_percent_position(&self, value: f64) {
        self.backend_ref().set_percent_position(value);
    }

    fn set_start(&self, percentage: f64) {
        self.backend_ref().set_start(percentage);
    }

    fn set_volume(&self, value: i64) {
        self.backend_ref().set_volume(value);
    }

    fn volume_scroll(&self, value: i64) {
        self.backend_ref().volume_scroll(value);
    }

    fn set_speed(&self, value: f64) {
        self.backend_ref().set_speed(value);
    }

    fn seek_forward(&self, value: i64) {
        self.backend_ref().seek_forward(value);
    }

    fn seek_backward(&self, value: i64) {
        self.backend_ref().seek_backward(value);
    }

    fn position(&self) -> BoxedFuture<'_, f64> {
        let backend = self.backend_rc();
        Box::pin(async move { backend.position().await })
    }

    fn paused(&self) -> BoxedFuture<'_, bool> {
        let backend = self.backend_rc();
        Box::pin(async move { backend.paused().await })
    }

    fn duration(&self) -> BoxedFuture<'_, f64> {
        let backend = self.backend_rc();
        Box::pin(async move { backend.duration().await })
    }

    fn set_aid(&self, value: TrackSelection) {
        self.backend_ref().set_aid(value);
    }

    fn set_sid(&self, value: TrackSelection) {
        self.backend_ref().set_sid(value);
    }

    fn disable_aid(&self) {
        self.backend_ref().disable_aid();
    }

    fn disable_sid(&self) {
        self.backend_ref().disable_sid();
    }

    fn set_keep_aspect_ratio(&self, keep: bool) {
        self.backend_ref().set_keep_aspect_ratio(keep);
    }

    fn set_slang(&self, value: String) {
        self.backend_ref().set_slang(value);
    }

    fn get_track_id(&self, kind: TrackKind) -> BoxedFuture<'_, i64> {
        let backend = self.backend_rc();
        Box::pin(async move { backend.get_track_id(kind).await })
    }

    fn press_key(&self, key: u32, state: ModifierType) {
        self.backend_ref().press_key(key, state);
    }

    fn release_key(&self, key: u32, state: ModifierType) {
        self.backend_ref().release_key(key, state);
    }

    fn display_stats_toggle(&self) {
        self.backend_ref().display_stats_toggle();
    }

    fn set_brightness(&self, value: f64) {
        self.backend_ref().set_brightness(value);
    }

    fn set_contrast(&self, value: f64) {
        self.backend_ref().set_contrast(value);
    }

    fn set_gamma(&self, value: f64) {
        self.backend_ref().set_gamma(value);
    }

    fn set_hue(&self, value: f64) {
        self.backend_ref().set_hue(value);
    }

    fn set_saturation(&self, value: f64) {
        self.backend_ref().set_saturation(value);
    }

    fn set_sub_pos(&self, value: f64) {
        self.backend_ref().set_sub_pos(value);
    }

    fn set_sub_font_size(&self, value: f64) {
        self.backend_ref().set_sub_font_size(value);
    }

    fn set_sub_scale(&self, value: f64) {
        self.backend_ref().set_sub_scale(value);
    }

    fn set_sub_speed(&self, value: f64) {
        self.backend_ref().set_sub_speed(value);
    }

    fn set_sub_delay(&self, value: f64) {
        self.backend_ref().set_sub_delay(value);
    }

    fn set_sub_bold(&self, value: bool) {
        self.backend_ref().set_sub_bold(value);
    }

    fn set_sub_italic(&self, value: bool) {
        self.backend_ref().set_sub_italic(value);
    }

    fn set_sub_font(&self, value: &str) {
        self.backend_ref().set_sub_font(value);
    }

    fn set_sub_color(&self, value: &str) {
        self.backend_ref().set_sub_color(value);
    }

    fn set_sub_border_color(&self, value: &str) {
        self.backend_ref().set_sub_border_color(value);
    }

    fn set_sub_back_color(&self, value: &str) {
        self.backend_ref().set_sub_back_color(value);
    }

    fn set_sub_border_style(&self, value: &str) {
        self.backend_ref().set_sub_border_style(value);
    }

    fn set_sub_border_size(&self, value: f64) {
        self.backend_ref().set_sub_border_size(value);
    }

    fn set_sub_shadow_offset(&self, value: f64) {
        self.backend_ref().set_sub_shadow_offset(value);
    }

    fn set_audio_delay(&self, value: f64) {
        self.backend_ref().set_audio_delay(value);
    }

    fn set_audio_channels(&self, value: &str) {
        self.backend_ref().set_audio_channels(value);
    }

    fn set_audio_pan(&self, value: &str) {
        self.backend_ref().set_audio_pan(value);
    }

    fn clear_audio_pan(&self) {
        self.backend_ref().clear_audio_pan();
    }

    fn set_scale(&self, value: &str) {
        self.backend_ref().set_scale(value);
    }

    fn set_deband(&self, value: bool) {
        self.backend_ref().set_deband(value);
    }

    fn set_deband_iterations(&self, value: i64) {
        self.backend_ref().set_deband_iterations(value);
    }

    fn set_deband_threshold(&self, value: i64) {
        self.backend_ref().set_deband_threshold(value);
    }

    fn set_deband_range(&self, value: i64) {
        self.backend_ref().set_deband_range(value);
    }

    fn set_deband_grain(&self, value: i64) {
        self.backend_ref().set_deband_grain(value);
    }

    fn set_deinterlace(&self, value: bool) {
        self.backend_ref().set_deinterlace(value);
    }

    fn set_hwdec(&self, value: &str) {
        self.backend_ref().set_hwdec(value);
    }

    fn set_panscan(&self, value: f64) {
        self.backend_ref().set_panscan(value);
    }

    fn set_stretch_image_subs_to_screen(&self, value: bool) {
        self.backend_ref().set_stretch_image_subs_to_screen(value);
    }

    fn set_demuxer_max_bytes(&self, value: &str) {
        self.backend_ref().set_demuxer_max_bytes(value);
    }

    fn set_cache_secs(&self, value: f64) {
        self.backend_ref().set_cache_secs(value);
    }
}
