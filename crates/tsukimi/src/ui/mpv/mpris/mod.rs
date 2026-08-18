use anyhow::Result;

use adw::subclass::prelude::{
    ObjectSubclassExt,
    ObjectSubclassIsExt,
};
use gtk::{
    self,
    glib,
    prelude::*,
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
use tracing::warn;

mod metadata;
mod track_list;

impl MPVPage {
    pub async fn initialize_mpris(&self, app_id: &str) -> Result<()> {
        if self.imp().mpris_server.get().is_some() {
            return Err(anyhow::anyhow!("Mpris server already initialized"));
        }

        let server = LocalServer::new_with_track_list(app_id, self.imp().obj().clone()).await?;

        // It cant panic here
        self.imp().mpris_server.set(server).unwrap();
        spawn(self.imp().mpris_server.get().unwrap().run());
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
                let Some(server) = imp.mpris_server() else {
                    return;
                };
                if let Err(err) = server.properties_changed(property).await {
                    warn!("Failed to emit properties changed: {}", err);
                }
            }
        ));
    }

    pub fn notify_mpris_seeked(&self, position: i64) {
        spawn(glib::clone!(
            #[weak(rename_to=obj)]
            self,
            async move {
                let Some(server) = obj.mpris_server() else {
                    return;
                };
                let signal = Signal::Seeked {
                    position: Time::from_secs(position),
                };
                if let Err(err) = server.emit(signal).await {
                    warn!("Failed to emit mpris_seeked: {}", err);
                }
            }
        ));
    }

    pub fn notify_mpris_track_changed(&self) {
        let Some(current_video) = self.current_video() else {
            return;
        };
        let metadata = self.metadata_for_video(&current_video);
        self.mpris_properties_changed([
            Property::Metadata(metadata.clone()),
            Property::CanPlay(true),
            Property::CanPause(true),
            Property::CanSeek(true),
            Property::CanGoNext(self.has_next_video()),
            Property::CanGoPrevious(self.has_previous_video()),
        ]);
        self.notify_mpris_art_changed(current_video, metadata);
    }

    pub fn notify_mpris_playing(&self) {
        self.mpris_properties_changed([Property::PlaybackStatus(PlaybackStatus::Playing)]);
    }

    pub fn notify_mpris_paused(&self) {
        self.mpris_properties_changed([Property::PlaybackStatus(PlaybackStatus::Paused)]);
    }

    pub fn notify_mpris_volume(&self, volume: Volume) {
        self.mpris_properties_changed([Property::Volume(volume.clamp(0.0, 1.0))]);
    }

    pub fn notify_mpris_stopped(&self) {
        self.mpris_properties_changed([
            Property::Metadata(Metadata::new()),
            Property::CanPlay(false),
            Property::CanPause(false),
            Property::CanSeek(false),
            Property::CanGoNext(false),
            Property::CanGoPrevious(false),
            Property::PlaybackStatus(PlaybackStatus::Stopped),
        ]);
    }

    pub fn notify_mpris_loop_status(&self, status: ListRepeatMode) {
        self.mpris_properties_changed([Property::LoopStatus(status.into())]);
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
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(self.fullscreened())
    }

    async fn set_fullscreen(&self, fullscreen: bool) -> zbus::Result<()> {
        self.set_fullscreened(fullscreen);
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
        self.on_next_video().await;
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.on_previous_video().await;
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.on_pause_update(true);
        self.mpv().pause(true);
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        let paused = self.imp().video.paused();
        self.on_pause_update(!paused);
        self.mpv().pause(!paused);
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.on_stop_clicked();
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.on_pause_update(false);
        self.mpv().pause(false);
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let offset_seconds = offset.abs().as_secs();
        if offset.is_positive() {
            self.imp().video.seek_forward(offset_seconds);
        } else {
            self.imp().video.seek_backward(offset_seconds);
        }
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        self.mpv().set_position(position.as_secs() as f64);
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("OpenUri is not supported".into()))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(if self.current_video().is_none() {
            PlaybackStatus::Stopped
        } else if self.imp().video.paused() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Playing
        })
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::None)
    }

    async fn set_loop_status(&self, _status: LoopStatus) -> zbus::Result<()> {
        Err(zbus::Error::from(fdo::Error::NotSupported(
            "SetLoopStatus is not supported".into(),
        )))
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(self.imp().playback_speed_adj.value())
    }

    async fn set_rate(&self, rate: PlaybackRate) -> zbus::Result<()> {
        self.mpv().set_speed(rate);
        Ok(())
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
        Ok(self.imp().volume_bar.level().clamp(0.0, 1.0))
    }

    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        let volume = (volume.clamp(0.0, 1.0) * 100.0).round();
        self.imp().volume_adj.set_value(volume);
        self.imp().video.set_volume(volume as i64);
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_secs(self.imp().video.position() as i64))
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(0.1)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(5.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(self.has_next_video())
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(self.has_previous_video())
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
