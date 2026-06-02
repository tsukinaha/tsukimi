use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib;
use mpris_server::{
    Metadata,
    Property,
    Time,
};

use crate::{
    ui::{
        mpv::page::MPVPage,
        provider::tu_item::TuItem,
    },
    utils::{
        get_image_with_cache,
        spawn,
    },
};

impl MPVPage {
    pub(super) fn metadata(&self) -> Metadata {
        let Some(video) = self.current_video() else {
            return Metadata::new();
        };
        let mut metadata = self.metadata_for_video(&video);
        if let Some(art_url) = self.imp().mpris_art_url.borrow().as_ref() {
            metadata.set_art_url(Some(art_url.clone()));
        }
        metadata
    }

    pub(super) fn metadata_for_video(&self, video: &TuItem) -> Metadata {
        let mut builder = Metadata::builder()
            .trackid(self.track_id_for_video(video))
            .title(Self::video_title(video));
        let duration = video.run_time_ticks() / 10_000_000;
        if duration > 0 {
            builder = builder.length(Time::from_secs(duration as i64));
        }
        if let Some(series_name) = video.series_name() {
            builder = builder.album(series_name);
        }
        if let Some(artists) = video.artists() {
            builder = builder.artist([artists]);
        }
        builder.build()
    }

    pub(super) fn notify_mpris_art_changed(&self, video: TuItem, mut metadata: Metadata) {
        let video_id = video.id();
        let image_id = video.primary_image_item_id().unwrap_or_else(|| video.id());
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let path = get_image_with_cache(image_id, "Primary".to_string(), None)
                    .await
                    .unwrap_or_default();
                if path.is_empty()
                    || obj
                        .current_video()
                        .is_none_or(|video| video.id() != video_id)
                {
                    return;
                }
                let art_url = format!("file://{path}");
                obj.imp().mpris_art_url.replace(Some(art_url.clone()));
                metadata.set_art_url(Some(art_url));
                obj.mpris_properties_changed([Property::Metadata(metadata)]);
            }
        ));
    }

    fn video_title(video: &TuItem) -> String {
        if let Some(series_name) = video.series_name() {
            format!(
                "{} - S{}E{}: {}",
                series_name,
                video.parent_index_number(),
                video.index_number(),
                video.name()
            )
        } else {
            video.name()
        }
    }
}
