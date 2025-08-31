use adw::subclass::prelude::{
    ObjectSubclassExt,
    ObjectSubclassIsExt,
};
use mpris_server::{
    LocalPlayerInterface,
    LocalRootInterface,
    LoopStatus,
    Metadata,
    PlaybackRate,
    PlaybackStatus,
    Time,
    TrackId,
    Volume,
    zbus::{
        self,
        fdo,
    },
};

use super::player::MusicPlayer;
use crate::{
    APP_ID,
    CLIENT_ID,
    gstl::player::imp::ListRepeatMode,
};

impl LocalRootInterface for MusicPlayer {
    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn raise(&self) -> fdo::Result<()> {
        dbg!("Attempted to raise");
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn quit(&self) -> fdo::Result<()> {
        dbg!("Attempted to quit");
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, fullscreen: bool) -> zbus::Result<()> {
        dbg!("Request to set fullscreen", fullscreen);
        Ok(())
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok(CLIENT_ID.to_string())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok(APP_ID.to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl LocalPlayerInterface for MusicPlayer {
    async fn next(&self) -> fdo::Result<()> {
        dbg!("Attempted to next");
        self.imp().next().await;
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        dbg!("Attempted to previous");
        self.imp().prev().await;
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        dbg!("Attempted to pause");
        self.imp().pause();
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        dbg!("Attempted to play/pause");
        self.imp().play_pause();
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        dbg!("Attempted to stop");
        self.imp().stop();
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        dbg!("Attempted to play");
        self.imp().prepre_play().await;
        Ok(())
    }

    async fn seek(&self, _offset: Time) -> fdo::Result<()> {
        dbg!("TODO: implement seek");
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        dbg!("Setting position");
        let position = position.as_millis() as f64 / 1000.0;
        self.imp().set_position(position);
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("OpenUri is not supported".into()))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(match self.imp().state() {
            gst::State::Playing => PlaybackStatus::Playing,
            gst::State::Paused => PlaybackStatus::Paused,
            gst::State::Null => PlaybackStatus::Stopped,
            _ => PlaybackStatus::Stopped,
        })
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(match self.imp().obj().repeat_mode() {
            ListRepeatMode::None => LoopStatus::None,
            ListRepeatMode::RepeatOne => LoopStatus::Track,
            ListRepeatMode::Repeat => LoopStatus::Playlist,
        })
    }

    async fn set_loop_status(&self, status: LoopStatus) -> zbus::Result<()> {
        let repeat_mode = match status {
            LoopStatus::None => ListRepeatMode::None,
            LoopStatus::Track => ListRepeatMode::RepeatOne,
            LoopStatus::Playlist => ListRepeatMode::Repeat,
        };
        self.set_repeat_mode(repeat_mode);
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> zbus::Result<()> {
        Err(zbus::Error::from(fdo::Error::NotSupported(
            "SetRate is not supported".into(),
        )))
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_shuffle(&self, _shuffle: bool) -> zbus::Result<()> {
        Err(zbus::Error::from(fdo::Error::NotSupported(
            "SetShuffle is not supported".into(),
        )))
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(self
            .imp()
            .obj()
            .active_core_song()
            .as_ref()
            .map_or_else(Metadata::new, |song| {
                Metadata::builder()
                    .album(song.album_id())
                    .title(song.name())
                    .artist([song.artist()])
                    .build()
            }))
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(1.0)
    }

    async fn set_volume(&self, _volume: Volume) -> zbus::Result<()> {
        Err(zbus::Error::from(fdo::Error::NotSupported(
            "SetVolume is not supported".into(),
        )))
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_micros(
            self.imp().get_position().useconds() as i64
        ))
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(self.imp().next_song().is_some())
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(self.imp().prev_song().is_some())
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(self.imp().active_core_song.borrow().is_some())
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(self.imp().active_core_song.borrow().is_some())
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(self.imp().active_core_song.borrow().is_some())
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
