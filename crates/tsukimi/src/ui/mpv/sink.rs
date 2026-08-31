use std::cell::Cell;

use crate::client::jellyfin_client::JELLYFIN_CLIENT;

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use glib::Object;
use gtk::{
    gio,
    glib,
};
use mutsumi::{
    ContextedMPV,
    MpvValue,
    MutsumiVideoPlayer,
    TrackKind,
    TrackSelection,
};
use tracing::info;

mod imp {
    use super::*;

    pub struct MPVPlaySink {
        pub player: MutsumiVideoPlayer,
        pub position: Cell<f64>,
        pub paused: Cell<bool>,
    }

    impl Default for MPVPlaySink {
        fn default() -> Self {
            Self {
                player: MutsumiVideoPlayer::new(),
                position: Cell::new(0.0),
                paused: Cell::new(true),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MPVPlaySink {
        const NAME: &'static str = "MPVPlaySink";
        type Type = super::MPVPlaySink;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for MPVPlaySink {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            self.player.set_hexpand(true);
            self.player.set_vexpand(true);
            obj.set_child(Some(&self.player));
        }

        fn dispose(&self) {
            self.player.unparent();
        }
    }

    impl WidgetImpl for MPVPlaySink {}
    impl BinImpl for MPVPlaySink {}
}

glib::wrapper! {
    pub struct MPVPlaySink(ObjectSubclass<imp::MPVPlaySink>)
        @extends adw::Bin, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget;
}

impl Default for MPVPlaySink {
    fn default() -> Self {
        Self::new()
    }
}

impl MPVPlaySink {
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
        let url = JELLYFIN_CLIENT.resolve_url(url);

        let (imp, player) = (self.imp(), self.player());
        info!("Now Playing: {}", url);

        imp.position.set(start_seconds);
        imp.paused.set(false);

        player.set_start(start_seconds);
        player.load_video(&url);
        player.pause(false);
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
