use std::cell::Cell;

use glib::Object;
use gtk::{
    gio,
    glib,
    prelude::*,
    subclass::prelude::*,
};
use mutsumi::{
    ContextedMPV,
    MpvValue,
    MutsumiVideoPlayer,
    TrackKind,
    TrackSelection,
};
use tracing::info;
use url::Url;

use super::options_matcher::{
    match_audio_channels,
    match_hwdec_interop,
    match_sub_border_style,
    match_video_upscale,
};
use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    ui::models::SETTINGS,
    utils::spawn,
};
use adw::{
    prelude::*,
    subclass::prelude::*,
};

const MAX_VOLUME: i64 = 100;

mod imp {
    use super::*;

    pub struct MPVGLArea {
        pub player: MutsumiVideoPlayer,
        pub position: Cell<f64>,
        pub paused: Cell<bool>,
    }

    impl Default for MPVGLArea {
        fn default() -> Self {
            Self {
                player: MutsumiVideoPlayer::new(),
                position: Cell::new(0.0),
                paused: Cell::new(true),
            }
        }
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
            self.player.set_hexpand(true);
            self.player.set_vexpand(true);
            obj.set_child(Some(&self.player));

            super::configure_mpv(&self.player);
        }

        fn dispose(&self) {
            self.player.unparent();
        }
    }

    impl WidgetImpl for MPVGLArea {}
    impl BinImpl for MPVGLArea {}
}

glib::wrapper! {
    pub struct MPVGLArea(ObjectSubclass<imp::MPVGLArea>)
        @extends adw::Bin, gtk::Widget,
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

    pub fn mpv(&self) -> &ContextedMPV {
        self.imp().player.mpv()
    }

    pub fn player(&self) -> &MutsumiVideoPlayer {
        &self.imp().player
    }

    pub fn play(&self, url: &str, start_seconds: f64) {
        let url = url.to_owned();

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let url = JELLYFIN_CLIENT.get_streaming_url(&url).await;

                info!("Now Playing: {}", url);
                obj.imp().position.set(start_seconds);
                obj.imp().paused.set(false);

                obj.player().set_start(start_seconds);
                obj.player().load_video(&url);
                obj.player().pause(false);
            }
        ));
    }

    pub fn add_sub(&self, url: &str) {
        self.player().add_sub(url)
    }

    pub fn seek_forward(&self, value: i64) {
        self.player().seek_forward(value)
    }

    pub fn seek_backward(&self, value: i64) {
        self.player().seek_backward(value)
    }

    pub fn set_position(&self, value: f64) {
        self.imp().position.set(value);
        self.player().set_position(value)
    }

    pub fn position(&self) -> f64 {
        self.imp().position.get()
    }

    pub fn update_position(&self, value: f64) {
        self.imp().position.set(value);
    }

    pub fn set_aid(&self, value: TrackSelection) {
        self.player().set_aid(value)
    }

    pub async fn get_track_id(&self, kind: TrackKind) -> i64 {
        self.player().get_track_id(kind).await
    }

    pub fn set_sid(&self, value: TrackSelection) {
        self.player().set_sid(value)
    }

    pub fn press_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.player().press_key(key, state)
    }

    pub fn release_key(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.player().release_key(key, state)
    }

    pub fn set_speed(&self, value: f64) {
        self.player().set_speed(value)
    }

    pub fn set_volume(&self, value: i64) {
        self.player().set_volume(value)
    }

    pub fn display_stats_toggle(&self) {
        self.player().display_stats_toggle()
    }

    pub fn paused(&self) -> bool {
        self.imp().paused.get()
    }

    pub fn update_paused(&self, value: bool) {
        self.imp().paused.set(value);
    }

    pub fn pause(&self) {
        self.player().command_pause();
    }

    pub fn volume_scroll(&self, value: i64) {
        self.player().volume_scroll(value)
    }

    pub fn set_slang(&self, value: String) {
        self.player().set_slang(value)
    }

    pub fn stop(&self) {
        self.imp().paused.set(true);
        self.player().stop();
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<MpvValue>,
    {
        self.player().set_property(property, value)
    }
}

fn configure_mpv(player: &MutsumiVideoPlayer) {
    if SETTINGS.mpv_config() {
        player.set_property("config", true);
        player.set_property("config-dir", SETTINGS.mpv_config_dir());
    }
    player.set_property("input-vo-keyboard", true);
    player.set_property("input-default-bindings", true);
    player.set_property("user-agent", crate::USER_AGENT.as_str());
    player.set_property("video-timing-offset", 0_i64);
    player.set_property("video-sync", "audio");
    player.set_property("osc", false);
    player.set_property("osd-level", 0_i64);
    player.set_demuxer_max_bytes(&format!("{}MiB", SETTINGS.mpv_cache_size()));
    player.set_cache_secs(SETTINGS.mpv_cache_time() as f64);
    player.set_property("volume-max", MAX_VOLUME);
    player.set_volume(SETTINGS.mpv_default_volume() as i64);
    player.set_sub_bold(SETTINGS.mpv_subtitle_bold());
    player.set_sub_italic(SETTINGS.mpv_subtitle_italic());
    player.set_sub_justify(match SETTINGS.mpv_subtitle_justify() {
        0 => "left",
        2 => "right",
        _ => "center",
    });
    player.set_sub_pos(SETTINGS.mpv_subtitle_position() as f64);
    player.set_sub_font_size(SETTINGS.mpv_subtitle_size() as f64);
    player.set_sub_scale(SETTINGS.mpv_subtitle_scale());
    player.set_sub_font(&SETTINGS.mpv_subtitle_font());
    player.set_sub_border_style(match_sub_border_style(SETTINGS.mpv_subtitle_border_style()));
    player.set_sub_border_size(SETTINGS.mpv_subtitle_border_size() as f64);
    player.set_sub_shadow_offset(SETTINGS.mpv_subtitle_shadow_offset() as f64);
    player.set_stretch_image_subs_to_screen(SETTINGS.mpv_subtitle_stretch_image_subs_to_screen());
    player.set_sub_color(&settings_color_to_mpv(
        SETTINGS.mpv_subtitle_text_color(),
        (1.0, 1.0, 1.0, 1.0),
    ));
    player.set_sub_border_color(&settings_color_to_mpv(
        SETTINGS.mpv_subtitle_border_color(),
        (0.0, 0.0, 0.0, 1.0),
    ));
    player.set_sub_back_color(&settings_color_to_mpv(
        SETTINGS.mpv_subtitle_background_color(),
        (0.0, 0.0, 0.0, 0.0),
    ));
    player.set_hwdec(match_hwdec_interop(SETTINGS.mpv_hwdec()));
    player.set_scale(match_video_upscale(SETTINGS.mpv_video_scale()));
    player.set_loop_file(if SETTINGS.mpv_action_after_video_end() == 1 {
        "inf"
    } else {
        "no"
    });
    player.set_audio_channels(match_audio_channels(SETTINGS.mpv_audio_channel()));
    if let Some(uri) = crate::client::proxy::get_proxy_settings() {
        let url = if Url::parse(&uri).is_ok() {
            uri
        } else {
            format!("http://{uri}")
        };
        player.set_property("http-proxy", url);
    }
    player.set_property(
        "alang",
        match SETTINGS.mpv_audio_preferred_lang() {
            0 => "",
            1 => "eng",
            2 => "chs",
            3 => "jpn",
            4 => "chi",
            5 => "ara",
            6 => "nob",
            7 => "por",
            8 => "fre",
            9 => "rus",
            _ => "",
        },
    );
}

fn settings_color_to_mpv(value: String, default: (f32, f32, f32, f32)) -> String {
    let rgba = gtk::gdk::RGBA::parse(&value)
        .unwrap_or_else(|_| gtk::gdk::RGBA::new(default.0, default.1, default.2, default.3));

    format!(
        "{}/{}/{}/{}",
        rgba.red(),
        rgba.green(),
        rgba.blue(),
        rgba.alpha()
    )
}
