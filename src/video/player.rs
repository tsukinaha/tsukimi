use glib::Object;
use gtk::{gdk::ModifierType, glib, subclass::prelude::*};

use crate::{MpvValue, MutsumiVideoSink};

use super::backend::{BoxedFuture, TrackKind, TrackSelection};

mod imp {
    use std::cell::Cell;

    use adw::prelude::*;
    use adw::subclass::prelude::*;
    use gtk::CssProvider;

    use crate::{MutsumiVideoSink, SIZE_CHANNEL};

    use super::*;
    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::MutsumiVideoPlayer)]
    pub struct MutsumiVideoPlayer {
        pub backend: MutsumiVideoSink,
        last_size: Cell<(i32, i32)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MutsumiVideoPlayer {
        const NAME: &'static str = "MutsumiVideoPlayer";
        type Type = super::MutsumiVideoPlayer;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MutsumiVideoPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_hexpand(true);
            obj.set_vexpand(true);

            let graphics_offload = gtk::GraphicsOffload::default();
            let picture = gtk::Picture::new();
            picture.set_hexpand(true);
            picture.set_vexpand(true);
            picture.set_paintable(Some(&self.backend));
            graphics_offload.set_child(Some(&picture));
            obj.set_child(Some(&graphics_offload));

            obj.add_css_class("mutsumi-video-player");

            let provider = CssProvider::new();
            provider.load_from_string(
                "
                .mutsumi-video-player {
                    background: black;
                }",
            );

            gtk::style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Could not connect to display"),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    impl WidgetImpl for MutsumiVideoPlayer {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);

            let factor = self.obj().scale_factor();
            let size = (width * factor, height * factor);
            if self.last_size.replace(size) != size {
                let _ = SIZE_CHANNEL.tx.send(size);
            }
        }
    }
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

    pub fn backend_ref(&self) -> &MutsumiVideoSink {
        let imp = self.imp();
        &imp.backend
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<MpvValue>,
    {
        self.backend_ref().mpv().mpv.set_property(property, value);
    }
}

impl MutsumiVideoPlayer {
    pub fn play(&self, url: &str, start_seconds: f64) {
        self.backend_ref().play(url, start_seconds);
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

    pub fn set_start(&self, start_seconds: f64) {
        self.backend_ref().set_start(start_seconds);
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
}
