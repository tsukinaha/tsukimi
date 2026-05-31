use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib;
use mpris_server::{
    LocalTrackListInterface,
    Metadata,
    TrackId,
    TrackListSignal,
    Uri,
    zbus::fdo,
};
use tracing::{
    info,
    warn,
};

use crate::{
    ui::{
        mpv::page::MPVPage,
        provider::tu_item::TuItem,
    },
    utils::spawn,
};

const TRACK_ID_PREFIX: &str = "/moe/tsuna/Tsukimi/TrackList/track_";

impl MPVPage {
    pub(crate) fn notify_mpris_track_list_replaced(&self) {
        spawn(glib::clone!(
            #[weak(rename_to=obj)]
            self,
            async move {
                match obj.mpris_server() {
                    Some(server) => {
                        let signal = TrackListSignal::TrackListReplaced {
                            tracks: obj.track_ids(),
                            current_track: obj.current_track_id(),
                        };
                        if let Err(err) = server.track_list_emit(signal).await {
                            warn!("Failed to emit mpris track list replaced: {}", err);
                        }
                    }
                    None => {
                        info!("Failed to get MPRIS server.");
                    }
                }
            }
        ));
    }

    pub(super) fn track_id_for_video(&self, video: &TuItem) -> TrackId {
        let video_list = self.imp().current_episode_list.borrow();
        video_list
            .iter()
            .position(|item| item.id() == video.id())
            .or_else(|| {
                video_list.iter().position(|item| {
                    item.index_number() == video.index_number()
                        && item.parent_index_number() == video.parent_index_number()
                })
            })
            .map(Self::track_id)
            .unwrap_or(TrackId::NO_TRACK)
    }

    pub(super) fn has_next_video(&self) -> bool {
        self.current_video_index()
            .is_some_and(|index| index + 1 < self.imp().current_episode_list.borrow().len())
    }

    pub(super) fn has_previous_video(&self) -> bool {
        self.current_video_index().is_some_and(|index| index > 0)
    }

    fn track_id(index: usize) -> TrackId {
        TrackId::try_from(format!("{TRACK_ID_PREFIX}{index}")).expect("valid MPRIS track id")
    }

    fn track_index(track_id: &TrackId) -> Option<usize> {
        track_id
            .as_str()
            .strip_prefix(TRACK_ID_PREFIX)
            .and_then(|index| index.parse().ok())
    }

    fn track_ids(&self) -> Vec<TrackId> {
        self.imp()
            .current_episode_list
            .borrow()
            .iter()
            .enumerate()
            .map(|(index, _)| Self::track_id(index))
            .collect()
    }

    fn current_video_index(&self) -> Option<usize> {
        let current_video = self.current_video()?;
        let video_list = self.imp().current_episode_list.borrow();

        video_list
            .iter()
            .position(|video| video.id() == current_video.id())
            .or_else(|| {
                video_list.iter().position(|video| {
                    video.index_number() == current_video.index_number()
                        && video.parent_index_number() == current_video.parent_index_number()
                })
            })
    }

    fn current_track_id(&self) -> TrackId {
        self.current_video_index()
            .map(Self::track_id)
            .unwrap_or(TrackId::NO_TRACK)
    }
}

impl LocalTrackListInterface for MPVPage {
    async fn get_tracks_metadata(&self, track_ids: Vec<TrackId>) -> fdo::Result<Vec<Metadata>> {
        let video_list = self.imp().current_episode_list.borrow();
        let metadata = track_ids
            .iter()
            .filter_map(Self::track_index)
            .filter_map(|index| video_list.get(index))
            .map(|video| self.metadata_for_video(video))
            .collect();
        Ok(metadata)
    }

    async fn add_track(
        &self, _uri: Uri, _after_track: TrackId, _set_as_current: bool,
    ) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "Editing the video queue is not supported".into(),
        ))
    }

    async fn remove_track(&self, _track_id: TrackId) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "Editing the video queue is not supported".into(),
        ))
    }

    async fn go_to(&self, track_id: TrackId) -> fdo::Result<()> {
        let item = Self::track_index(&track_id)
            .and_then(|index| self.imp().current_episode_list.borrow().get(index).cloned());

        if let Some(item) = item {
            self.in_play_item(item).await;
        }

        Ok(())
    }

    async fn tracks(&self) -> fdo::Result<Vec<TrackId>> {
        Ok(self.track_ids())
    }

    async fn can_edit_tracks(&self) -> fdo::Result<bool> {
        Ok(false)
    }
}
