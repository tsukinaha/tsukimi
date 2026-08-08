use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{
    gio::prelude::FileExt,
    glib,
};
use mpris_server::{
    Metadata,
    Property,
    Time,
};

use crate::{
    ui::{
        mpv::page::MPVPage,
        provider::tu_item::TuItem,
        widgets::tu_item::{
            CardOptions,
            select_picture_source,
        },
    },
    utils::{
        resolve_picture_file,
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
        let mut builder = Metadata::builder().trackid(self.track_id_for_video(video));
        let duration = video.run_time_ticks() / 10_000_000;
        if duration > 0 {
            builder = builder.length(Time::from_secs(duration as i64));
        }
        if let Some(series_name) = video.series_name() {
            builder = builder
                .title(format!(
                    "S{}E{}: {}",
                    video.parent_index_number(),
                    video.index_number(),
                    video.name()
                ))
                .album(series_name);
        } else {
            builder = builder.title(video.name());
        }
        if let Some(artists) = video.artists() {
            builder = builder.artist([artists]);
        }
        builder.build()
    }

    pub(super) fn notify_mpris_art_changed(&self, video: TuItem, mut metadata: Metadata) {
        let video_id = video.id();
        let source = select_picture_source(&video, CardOptions::default());
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                if obj
                    .current_video()
                    .is_none_or(|video| video.id() != video_id)
                {
                    return;
                }
                if let Some(source) = source
                    && let Ok(file) = resolve_picture_file(source).await
                {
                    let art_url = file.uri().to_string();
                    obj.imp().mpris_art_url.replace(Some(art_url.clone()));
                    metadata.set_art_url(Some(art_url));
                } else {
                    obj.imp().mpris_art_url.replace(None);
                }
                obj.mpris_properties_changed([Property::Metadata(metadata)]);
            }
        ));
    }
}
