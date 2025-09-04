use anyhow::Result;

use adw::subclass::prelude::{
    ObjectSubclassExt,
    ObjectSubclassIsExt,
};
use gtk::{
    self,
    glib,
};
use mpris_server::{
    LocalPlayerInterface,
    LocalRootInterface,
    LocalServer,
    LoopStatus,
    Metadata,
    PlaybackRate,
    PlaybackStatus,
    Property,
    Signal,
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
    utils::{
        get_image_with_cache,
        spawn,
    },
};
use tracing::{
    info,
    warn,
};

impl MusicPlayer {
    pub async fn initialize_mpris(&self) -> Result<()> {
        let server = LocalServer::new(APP_ID, self.imp().obj().clone()).await?;
        spawn(server.run());
        self.imp()
            .mpris_server
            .set(server)
            .map_err(|_| anyhow::anyhow!("Mpris server already initialized"))?;
        Ok(())
    }

    pub fn mpris_server(&self) -> Option<&LocalServer<MusicPlayer>> {
        self.imp().mpris_server.get()
    }

    pub fn mpris_properties_changed(&self, property: impl IntoIterator<Item = Property> + 'static) {
        spawn(glib::clone!(
            #[weak(rename_to=imp)]
            self,
            async move {
                match imp.mpris_server() {
                    Some(server) => {
                        if let Err(err) = server.properties_changed(property).await {
                            warn!("Failed to emit properties changed: {}", err);
                        }
                    }
                    None => {
                        info!("Failed to get MPRIS server.");
                    }
                }
            }
        ));
    }

    pub fn notify_mpris_seeked(&self, position: i64) {
        spawn(glib::clone!(
            #[weak(rename_to=obj)]
            self,
            async move {
                match obj.mpris_server() {
                    Some(server) => {
                        let signal = Signal::Seeked {
                            position: Time::from_millis(position),
                        };
                        if let Err(err) = server.emit(signal).await {
                            warn!("Failed to emit mpris_seeked: {}", err);
                        }
                    }
                    None => {
                        info!("Failed to get MPRIS server.");
                    }
                }
            }
        ));
    }

    pub fn notify_mpris_song_changed(&self, has_prev: bool, has_next: bool) {
        self.mpris_properties_changed([
            Property::Metadata(self.metadata().clone()),
            Property::CanGoPrevious(has_prev),
            Property::CanGoNext(has_next),
        ]);
        self.notify_mpris_art_changed();
    }

    pub fn notify_mpris_playing(&self) {
        self.mpris_properties_changed([
            Property::CanPlay(true),
            Property::CanPause(true),
            Property::CanSeek(true),
            Property::PlaybackStatus(PlaybackStatus::Playing),
        ]);
    }

    pub fn notify_mpris_paused(&self) {
        self.mpris_properties_changed([
            Property::CanPlay(true),
            Property::CanPause(false),
            Property::CanSeek(true),
            Property::PlaybackStatus(PlaybackStatus::Paused),
        ]);
    }

    pub fn notify_mpris_stopped(&self) {
        self.mpris_properties_changed([
            Property::CanPlay(true),
            Property::CanPause(false),
            Property::CanSeek(false),
            Property::PlaybackStatus(PlaybackStatus::Stopped),
        ]);
    }

    pub fn notify_mpris_loop_status(&self, status: ListRepeatMode) {
        self.mpris_properties_changed([Property::LoopStatus(status.into())]);
    }

    pub fn notify_mpris_art_changed(&self) {
        let mut metadata = self.metadata().clone();
        spawn(glib::clone!(
            #[weak(rename_to = imp)]
            self,
            async move {
                if let Some(core_song) = imp.active_core_song().as_ref() {
                    let id = if core_song.have_single_track_image() {
                        core_song.id()
                    } else {
                        core_song.album_id()
                    };
                    let path = get_image_with_cache(id, "Primary".to_string(), None)
                        .await
                        .unwrap_or_default();
                    let url = format!("file://{}", path);
                    metadata.set_art_url(Some(url));
                    imp.mpris_properties_changed([Property::Metadata(metadata)]);
                }
            }
        ));
    }

    pub fn metadata(&self) -> Metadata {
        self.imp()
            .obj()
            .active_core_song()
            .as_ref()
            .map_or_else(Metadata::new, |song| {
                Metadata::builder()
                    .album(song.album_id())
                    .title(song.name())
                    .length(Time::from_secs(song.duration() as i64))
                    .artist([song.artist()])
                    .build()
            })
    }
}

impl LocalRootInterface for MusicPlayer {
    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn raise(&self) -> fdo::Result<()> {
        crate::mpris_common::raise_window().await
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn quit(&self) -> fdo::Result<()> {
        crate::mpris_common::quit_application().await
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> zbus::Result<()> {
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
        self.imp().next().await;
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.imp().prev().await;
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.imp().pause();
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.imp().play_pause();
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.imp().stop();
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.imp().prepre_play().await;
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let position = self.imp().position() + offset.as_secs() as f64;
        self.imp().set_position(position);
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        let position = position.as_secs() as f64;
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
        Ok(self.imp().obj().repeat_mode().into())
    }

    async fn set_loop_status(&self, status: LoopStatus) -> zbus::Result<()> {
        self.set_repeat_mode(ListRepeatMode::from(status));
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
        Ok(self.metadata())
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(1.0)
    }

    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        self.imp().set_volume(volume);
        Ok(())
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

impl From<ListRepeatMode> for LoopStatus {
    fn from(mode: ListRepeatMode) -> Self {
        match mode {
            ListRepeatMode::None => LoopStatus::None,
            ListRepeatMode::RepeatOne => LoopStatus::Track,
            ListRepeatMode::Repeat => LoopStatus::Playlist,
        }
    }
}

impl From<LoopStatus> for ListRepeatMode {
    fn from(status: LoopStatus) -> Self {
        match status {
            LoopStatus::None => ListRepeatMode::None,
            LoopStatus::Track => ListRepeatMode::RepeatOne,
            LoopStatus::Playlist => ListRepeatMode::Repeat,
        }
    }
}
