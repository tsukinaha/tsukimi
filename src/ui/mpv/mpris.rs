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

use crate::{
    APP_ID,
    CLIENT_ID,
    gstl::player::imp::ListRepeatMode,
    ui::mpv::page::MPVPage,
    utils::spawn,
};
use tracing::{
    debug,
    info,
    warn,
};

impl MPVPage {
    pub async fn initialize_mpris(&self, app_id: &str) -> Result<()> {
        let server = LocalServer::new(app_id, self.imp().obj().clone()).await?;
        spawn(server.run());
        self.imp()
            .mpris_server
            .set(server)
            .map_err(|_| anyhow::anyhow!("Mpris server already initialized"))?;
        Ok(())
    }

    pub fn mpris_server(&self) -> Option<&LocalServer<MPVPage>> {
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

    pub fn notify_mpris_media_changed(&self) {
        self.mpris_properties_changed([Property::Metadata(self.metadata().clone())]);
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

    pub fn notify_mpris_has_chapters(&self, has_chapters: bool) {
        self.mpris_properties_changed([
            Property::CanGoNext(has_chapters),
            Property::CanGoPrevious(has_chapters),
        ]);
    }

    pub fn notify_mpris_art_changed(&self) {}

    pub fn metadata(&self) -> Metadata {
        self.imp()
            .obj()
            .current_video()
            .as_ref()
            .map_or_else(Metadata::new, |video| {
                dbg!(&video.poster());
                Metadata::builder()
                    .title(video.name())
                    .artist([video.artists().unwrap_or_default()])
                    .art_url(video.poster().unwrap_or_default())
                    .build()
            })
    }
}

impl LocalRootInterface for MPVPage {
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

impl LocalPlayerInterface for MPVPage {
    async fn next(&self) -> fdo::Result<()> {
        self.chapter_next();
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.chapter_next();
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.on_pause_update(true);
        self.mpv().pause(true);
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        if self.imp().video.paused() {
            self.on_pause_update(false);
            self.mpv().pause(false);
        } else {
            self.on_pause_update(true);
            self.mpv().pause(true);
        }
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        // TODO implement
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.on_pause_update(false);
        self.mpv().pause(false);
        Ok(())
    }

    async fn seek(&self, _offset: Time) -> fdo::Result<()> {
        debug!("TODO: implement seek");
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, _position: Time) -> fdo::Result<()> {
        // TODO implement
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("OpenUri is not supported".into()))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(PlaybackStatus::Stopped)
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        // TODO implement
        Ok(LoopStatus::None)
    }

    async fn set_loop_status(&self, _status: LoopStatus) -> zbus::Result<()> {
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

    async fn set_volume(&self, _volume: Volume) -> zbus::Result<()> {
        Err(zbus::Error::from(fdo::Error::NotSupported(
            "SetVolume is not supported".into(),
        )))
    }

    async fn position(&self) -> fdo::Result<Time> {
        //TODO implement
        Ok(Time::from_micros(123))
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(self.current_video().is_some())
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(self.current_video().is_some())
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(self.current_video().is_some())
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(self.current_video().is_some())
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(self.current_video().is_some())
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
