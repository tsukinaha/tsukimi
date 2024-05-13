use std::cell::RefCell;

use crate::ui::{provider::core_song::CoreSong, widgets::song_widget::State};

use super::MUSIC_PLAYER;

pub struct Player {
    core_song: RefCell<Option<CoreSong>>,
}

impl Player {
    pub fn from_core_song(core_song: CoreSong) -> Self {
        Self {
            core_song: RefCell::new(Some(core_song)),
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn play(&self, core_song: CoreSong) {
        if let Some(core_song_old) = self.core_song.borrow().as_ref() {
            if core_song_old != &core_song {
                core_song_old.set_state(State::Played);
            }
        }
        MUSIC_PLAYER.play(&core_song);
        self.core_song.replace(Some(core_song));
    }

    pub fn pause(&self) {
        MUSIC_PLAYER.pause();
    }

    pub fn stop(&self) {
        if let Some(core_song) = self.core_song.borrow().as_ref() {
            core_song.set_state(State::Played);
        }
        MUSIC_PLAYER.stop();
        self.core_song.replace(None);
    }

    pub fn unpause(&self) {
        MUSIC_PLAYER.unpause();
    }

    pub fn state(&self) -> gst::State {
        if self.core_song.borrow().as_ref().is_some() {
            MUSIC_PLAYER.state()
        } else {
            gst::State::Null
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {
            core_song: RefCell::new(None),
        }
    }
}
