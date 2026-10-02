use adw::{
    prelude::*,
    subclass::prelude::*,
};
use glib::Object;
use gtk::{
    gdk::ModifierType,
    glib,
    subclass::prelude::*,
};

use crate::{
    ContextedMPV,
    MPVGLArea,
    MpvValue,
    PlayParams,
};

use super::backend::{
    BoxedFuture,
    TrackKind,
    TrackSelection,
};

mod imp {
    use super::*;

    pub struct MutsumiVideoPlayer {
        pub backend: MPVGLArea,
    }

    impl Default for MutsumiVideoPlayer {
        fn default() -> Self {
            Self {
                backend: MPVGLArea::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MutsumiVideoPlayer {
        const NAME: &'static str = "MutsumiVideoPlayer";
        type Type = super::MutsumiVideoPlayer;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for MutsumiVideoPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            self.backend.set_hexpand(true);
            self.backend.set_vexpand(true);
            self.obj().set_child(Some(&self.backend));
            self.obj().add_css_class("mutsumi-video-player");

            let provider = gtk::CssProvider::new();
            provider.load_from_string(
                "
                .mutsumi-video-player {
                    background: black;
                }",
            );

            if let Some(display) = gtk::gdk::Display::default() {
                gtk::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
        }

        fn dispose(&self) {
            self.backend.shutdown();
            self.obj().set_child(None::<&gtk::Widget>);
        }
    }

    impl WidgetImpl for MutsumiVideoPlayer {}
    impl BinImpl for MutsumiVideoPlayer {}
}

glib::wrapper! {
    pub struct MutsumiVideoPlayer(ObjectSubclass<imp::MutsumiVideoPlayer>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for MutsumiVideoPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MutsumiVideoPlayer {
    pub fn new() -> Self {
        Object::new()
    }

    pub fn update_viewport(&self, _width: i32, _height: i32) {}

    fn backend_ref(&self) -> &MPVGLArea {
        &self.imp().backend
    }

    pub fn mpv(&self) -> &ContextedMPV {
        self.backend_ref().mpv()
    }

    pub fn play(&self, source: &PlayParams) {
        self.backend_ref().play(source.clone());
    }

    pub fn shutdown(&self) {
        self.backend_ref().shutdown();
    }

    pub fn stop(&self) {
        self.backend_ref().stop();
    }

    pub fn load_video(&self, url: &str) {
        self.backend_ref().load_video(url);
    }

    pub fn push_an_empty_texture(&self) {
        self.backend_ref().queue_render();
    }

    pub fn set_loop_playlist(&self, loop_: &str) {
        self.mpv().set_loop_playlist(loop_);
    }

    pub fn set_loop_file(&self, loop_: &str) {
        self.mpv().set_loop_file(loop_);
    }

    pub fn playlist_shuffle(&self) {
        self.mpv().playlist_shuffle();
    }

    pub fn playlist_unshuffle(&self) {
        self.mpv().playlist_unshuffle();
    }

    pub fn set_playlist(&self, urls: &[String]) {
        self.mpv().set_playlist(urls);
    }

    pub fn set_playlist_pos(&self, pos: i64) {
        self.mpv().set_playlist_pos(pos);
    }

    pub fn playlist_add(&self, url: &str, index: i64) {
        self.mpv().playlist_add(url, index);
    }

    pub fn playlist_remove(&self, index: i64) {
        self.mpv().playlist_remove(index);
    }

    pub fn playlist_move(&self, from: i64, to: i64) {
        self.mpv().playlist_move(from, to);
    }

    pub fn add_sub(&self, url: &str) {
        self.backend_ref().add_sub(url);
    }

    pub fn pause(&self, pause: bool) {
        self.backend_ref().pause(pause);
    }

    pub fn command_pause(&self) {
        self.backend_ref().command_pause();
    }

    pub fn set_position(&self, value: f64) {
        self.backend_ref().set_position(value);
    }

    pub fn set_percent_position(&self, value: f64) {
        self.backend_ref().set_percent_position(value);
    }

    pub fn set_start_time(&self, second: u64) {
        self.mpv().set_start_time(second);
    }

    pub fn set_start(&self, second: f64) {
        self.mpv().set_start(second);
    }

    pub fn set_volume(&self, value: i64) {
        self.backend_ref().set_volume(value);
    }

    pub fn volume_scroll(&self, value: i64) {
        self.backend_ref().volume_scroll(value);
    }

    pub fn set_speed(&self, value: f64) {
        self.backend_ref().set_speed(value);
    }

    pub fn seek_forward(&self, value: i64) {
        self.backend_ref().seek_forward(value);
    }

    pub fn seek_backward(&self, value: i64) {
        self.backend_ref().seek_backward(value);
    }

    pub async fn position(&self) -> f64 {
        self.backend_ref().position().await
    }

    pub async fn paused(&self) -> bool {
        self.backend_ref().paused().await
    }

    pub async fn duration(&self) -> f64 {
        self.backend_ref().duration().await
    }

    pub fn set_aid(&self, value: TrackSelection) {
        self.backend_ref().set_aid(value);
    }

    pub fn set_sid(&self, value: TrackSelection) {
        self.backend_ref().set_sid(value);
    }

    pub fn disable_aid(&self) {
        self.backend_ref().disable_aid();
    }

    pub fn disable_sid(&self) {
        self.backend_ref().disable_sid();
    }

    pub fn set_keep_aspect_ratio(&self, keep: bool) {
        self.backend_ref().set_keep_aspect_ratio(keep);
    }

    pub fn set_slang(&self, value: String) {
        self.backend_ref().set_slang(value);
    }

    pub fn get_track_id(&self, kind: TrackKind) -> BoxedFuture<'_, i64> {
        let backend = self.backend_ref().clone();
        Box::pin(async move { backend.get_track_id(kind).await })
    }

    pub fn press_key(&self, key: u32, state: ModifierType) {
        self.backend_ref().press_key(key, state);
    }

    pub fn release_key(&self, key: u32, state: ModifierType) {
        self.backend_ref().release_key(key, state);
    }

    pub fn display_stats_toggle(&self) {
        self.backend_ref().display_stats_toggle();
    }

    pub fn set_brightness(&self, value: f64) {
        self.backend_ref().set_brightness(value);
    }

    pub fn set_contrast(&self, value: f64) {
        self.backend_ref().set_contrast(value);
    }

    pub fn set_gamma(&self, value: f64) {
        self.backend_ref().set_gamma(value);
    }

    pub fn set_hue(&self, value: f64) {
        self.backend_ref().set_hue(value);
    }

    pub fn set_saturation(&self, value: f64) {
        self.backend_ref().set_saturation(value);
    }

    pub fn set_sub_pos(&self, value: f64) {
        self.backend_ref().set_sub_pos(value);
    }

    pub fn set_sub_font_size(&self, value: f64) {
        self.backend_ref().set_sub_font_size(value);
    }

    pub fn set_sub_scale(&self, value: f64) {
        self.backend_ref().set_sub_scale(value);
    }

    pub fn set_sub_speed(&self, value: f64) {
        self.backend_ref().set_sub_speed(value);
    }

    pub fn set_sub_delay(&self, value: f64) {
        self.backend_ref().set_sub_delay(value);
    }

    pub fn set_sub_justify(&self, value: &str) {
        self.mpv().mpv.set_property("sub-justify", value.to_owned());
    }

    pub fn set_sub_bold(&self, value: bool) {
        self.backend_ref().set_sub_bold(value);
    }

    pub fn set_sub_italic(&self, value: bool) {
        self.backend_ref().set_sub_italic(value);
    }

    pub fn set_sub_font(&self, value: &str) {
        self.backend_ref().set_sub_font(value);
    }

    pub fn set_sub_color(&self, value: &str) {
        self.backend_ref().set_sub_color(value);
    }

    pub fn set_sub_border_color(&self, value: &str) {
        self.backend_ref().set_sub_border_color(value);
    }

    pub fn set_sub_back_color(&self, value: &str) {
        self.backend_ref().set_sub_back_color(value);
    }

    pub fn set_sub_border_style(&self, value: &str) {
        self.backend_ref().set_sub_border_style(value);
    }

    pub fn set_sub_border_size(&self, value: f64) {
        self.backend_ref().set_sub_border_size(value);
    }

    pub fn set_sub_shadow_offset(&self, value: f64) {
        self.backend_ref().set_sub_shadow_offset(value);
    }

    pub fn set_audio_delay(&self, value: f64) {
        self.backend_ref().set_audio_delay(value);
    }

    pub fn set_audio_channels(&self, value: &str) {
        self.backend_ref().set_audio_channels(value);
    }

    pub fn set_audio_pan(&self, value: &str) {
        self.backend_ref().set_audio_pan(value);
    }

    pub fn clear_audio_pan(&self) {
        self.backend_ref().clear_audio_pan();
    }

    pub fn set_scale(&self, value: &str) {
        self.backend_ref().set_scale(value);
    }

    pub fn set_deband(&self, value: bool) {
        self.backend_ref().set_deband(value);
    }

    pub fn set_deband_iterations(&self, value: i64) {
        self.backend_ref().set_deband_iterations(value);
    }

    pub fn set_deband_threshold(&self, value: i64) {
        self.backend_ref().set_deband_threshold(value);
    }

    pub fn set_deband_range(&self, value: i64) {
        self.backend_ref().set_deband_range(value);
    }

    pub fn set_deband_grain(&self, value: i64) {
        self.backend_ref().set_deband_grain(value);
    }

    pub fn set_deinterlace(&self, value: bool) {
        self.backend_ref().set_deinterlace(value);
    }

    pub fn set_hwdec(&self, value: &str) {
        self.backend_ref().set_hwdec(value);
    }

    pub fn set_panscan(&self, value: f64) {
        self.backend_ref().set_panscan(value);
    }

    pub fn set_stretch_image_subs_to_screen(&self, value: bool) {
        self.backend_ref().set_stretch_image_subs_to_screen(value);
    }

    pub fn set_demuxer_max_bytes(&self, value: &str) {
        self.backend_ref().set_demuxer_max_bytes(value);
    }

    pub fn set_cache_secs(&self, value: f64) {
        self.backend_ref().set_cache_secs(value);
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<MpvValue>,
    {
        self.mpv().mpv.set_property(property, value);
    }
}
