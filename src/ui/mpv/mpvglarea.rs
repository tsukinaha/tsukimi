use std::cell::Cell;

use adw::prelude::*;
use glib::Object;
use gtk::{
    gio,
    glib,
    subclass::prelude::*,
};
use mutsumi::{
    MpvValue,
    TrackSelection,
    match_audio_channels,
    match_hwdec_interop,
    match_video_upscale,
};
use tracing::info;
use url::Url;

use crate::{
    USER_AGENT,
    client::proxy::get_proxy_settings,
    ui::models::SETTINGS,
};

mod imp {
    use adw::subclass::prelude::*;
    use gtk::glib;
    use mutsumi::MutsumiVideoPlayer;

    use super::*;

    #[derive(Default)]
    pub struct MPVGLArea {
        pub player: MutsumiVideoPlayer,
        pub position: Cell<f64>,
        pub paused: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MPVGLArea {
        const NAME: &'static str = "MPVGLArea";
        type Type = super::MPVGLArea;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for MPVGLArea {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_hexpand(true);
            obj.set_vexpand(true);
            obj.set_child(Some(&self.player));
            obj.apply_initial_settings();
        }

        fn dispose(&self) {
            self.player.shutdown();
        }
    }

    impl WidgetImpl for MPVGLArea {}
    impl BinImpl for MPVGLArea {}
}

glib::wrapper! {
    pub struct MPVGLArea(ObjectSubclass<imp::MPVGLArea>)
        @extends gtk::Widget, adw::Bin,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget;
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

    pub fn player(&self) -> mutsumi::MutsumiVideoPlayer {
        self.imp().player.clone()
    }

    fn apply_initial_settings(&self) {
        if SETTINGS.mpv_config() {
            self.set_property("config", true);
            self.set_property("config-dir", SETTINGS.mpv_config_dir());
        }

        self.set_property("input-vo-keyboard", true);
        self.set_property("input-default-bindings", true);
        self.set_property("user-agent", USER_AGENT.as_str());
        self.set_property("video-timing-offset", 0_i64);
        self.set_property("video-sync", "audio");
        self.set_property(
            "demuxer-max-bytes",
            format!("{}MiB", SETTINGS.mpv_cache_size()),
        );
        self.set_property("cache-secs", SETTINGS.mpv_cache_time() as i64);
        self.set_property("volume-max", 100_i64);
        self.set_property("volume", SETTINGS.mpv_default_volume() as i64);
        self.set_property("sub-font-size", SETTINGS.mpv_subtitle_size() as i64);
        self.set_property("sub-font", SETTINGS.mpv_subtitle_font());
        self.set_property("sub-scale", SETTINGS.mpv_subtitle_scale());
        self.set_property("hwdec", match_hwdec_interop(SETTINGS.mpv_hwdec()));
        self.set_property("scale", match_video_upscale(SETTINGS.mpv_video_scale()));
        self.set_property(
            "loop",
            if SETTINGS.mpv_action_after_video_end() == 1 {
                "inf"
            } else {
                "no"
            },
        );
        self.set_property(
            "audio-channels",
            match_audio_channels(SETTINGS.mpv_audio_channel()),
        );
        if let Some(uri) = get_proxy_settings() {
            let url =
                Url::parse(&uri).map_or_else(|_| format!("http://{uri}"), |_| uri.to_string());
            self.set_property("http-proxy", url);
        }
        let alang = match SETTINGS.mpv_audio_preferred_lang() {
            0 => "",
            1 => "eng",
            2 => "chs",
            3 => "jpn",
            4 => "chi",
            5 => "ara",
            6 => "nob",
            7 => "por",
            8 => "fre",
            _ => "",
        };
        self.set_property("alang", alang);
    }

    pub fn play(&self, url: &str, start_seconds: f64) {
        info!("Now Playing: {}", url);
        self.imp().position.set(start_seconds);
        self.imp().paused.set(false);
        self.imp().player.play(url, start_seconds);
    }

    pub fn add_sub(&self, url: &str) {
        self.imp().player.add_sub(url)
    }

    pub fn seek_forward(&self, value: i64) {
        self.imp().position.set(self.position() + value as f64);
        self.imp().player.seek_forward(value)
    }

    pub fn seek_backward(&self, value: i64) {
        self.imp()
            .position
            .set((self.position() - value as f64).max(0.0));
        self.imp().player.seek_backward(value)
    }

    pub fn set_position(&self, value: f64) {
        self.imp().position.set(value);
        self.imp().player.set_position(value)
    }

    pub fn set_cached_position(&self, value: f64) {
        self.imp().position.set(value);
    }

    pub fn position(&self) -> f64 {
        self.imp().position.get()
    }

    pub fn set_aid(&self, value: TrackSelection) {
        self.imp().player.set_aid(value)
    }

    pub fn set_sid(&self, value: TrackSelection) {
        self.imp().player.set_sid(value)
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.imp().player.press_key(key, state)
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.imp().player.release_key(key, state)
    }

    pub fn set_speed(&self, value: f64) {
        self.imp().player.set_speed(value)
    }

    pub fn set_volume(&self, value: i64) {
        self.imp().player.set_volume(value)
    }

    pub fn display_stats_toggle(&self) {
        self.imp().player.display_stats_toggle()
    }

    pub fn paused(&self) -> bool {
        self.imp().paused.get()
    }

    pub fn set_cached_paused(&self, paused: bool) {
        self.imp().paused.set(paused);
    }

    pub fn set_paused(&self, paused: bool) {
        self.imp().paused.set(paused);
        self.imp().player.pause(paused);
    }

    pub fn command_pause(&self) {
        self.imp().paused.set(!self.paused());
        self.imp().player.command_pause();
    }

    pub fn stop(&self) {
        self.imp().player.stop();
    }

    pub fn volume_scroll(&self, value: i64) {
        self.imp().player.volume_scroll(value)
    }

    pub fn set_slang(&self, value: String) {
        self.imp().player.set_slang(value)
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<MpvValue>,
    {
        self.imp()
            .player
            .backend_ref()
            .mpv()
            .mpv
            .set_property(property, value)
    }
}
