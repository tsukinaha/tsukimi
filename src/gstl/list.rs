use super::MUSIC_PLAYER;
use crate::ui::{provider::core_song::CoreSong, widgets::song_widget::State};
use gtk::{glib, subclass::prelude::*};

mod imp {
    use std::cell::RefCell;

    use gtk::{glib, prelude::*, subclass::prelude::*};

    use crate::ui::provider::core_song::CoreSong;
    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Player)]
    pub struct Player {
        #[property(get, set)]
        pub core_song: RefCell<Option<CoreSong>>,
    }

    impl ObjectImpl for Player {}

    #[glib::object_subclass]
    impl ObjectSubclass for Player {
        const NAME: &'static str = "Player";
        type Type = super::Player;
    }
}

glib::wrapper! {
    pub struct Player(ObjectSubclass<imp::Player>);
}

impl Player {
    pub fn new() -> Player {
        glib::Object::builder().build()
    }

    pub fn play(&self, core_song: CoreSong) {
        let imp = self.imp();
        if let Some(core_song_old) = imp.core_song.borrow().as_ref() {
            if core_song_old != &core_song {
                core_song_old.set_state(State::Played);
            }
        }
        MUSIC_PLAYER.play(&core_song);
        imp.core_song.replace(Some(core_song));
    }

    pub fn pause(&self) {
        MUSIC_PLAYER.pause();
    }

    pub fn set_position(&self, position: f64) {
        MUSIC_PLAYER.set_position(position);
    }

    pub fn stop(&self) {
        let imp = self.imp();
        if let Some(core_song) = imp.core_song.borrow().as_ref() {
            core_song.set_state(State::Played);
        }
        MUSIC_PLAYER.stop();
        imp.core_song.replace(None);
    }

    pub fn unpause(&self) {
        MUSIC_PLAYER.unpause();
    }

    pub fn state(&self) -> gst::State {
        if self.imp().core_song.borrow().as_ref().is_some() {
            MUSIC_PLAYER.state()
        } else {
            gst::State::Null
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
