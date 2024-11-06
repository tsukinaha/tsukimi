use std::cell::RefCell;

use adw::prelude::*;
use gettextrs::gettext;
use glib::DateTime;
use gtk::{
    glib,
    glib::subclass::prelude::*,
};

use crate::{
    client::{
        client::EMBY_CLIENT,
        error::UserFacingError,
        structs::{
            Back,
            SimpleListItem,
        },
    },
    toast,
    ui::widgets::{
        item::ItemPage,
        list::ListPage,
        music_album::AlbumPage,
        other::OtherPage,
        single_grid::{
            imp::ListType,
            SingleGrid,
        },
        window::Window,
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};

#[derive(Default, Clone)]
struct AlbumArtist {
    name: String,
    id: String,
}

pub mod imp {
    use glib::DateTime;
    use gtk::glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::TuItem)]
    pub struct TuItem {
        #[property(get, set)]
        id: RefCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        index_number: RefCell<u32>,
        #[property(get, set)]
        parent_index_number: RefCell<u32>,
        #[property(get, set)]
        series_name: RefCell<Option<String>>,
        #[property(get, set)]
        series_id: RefCell<Option<String>>,
        #[property(get, set)]
        played_percentage: RefCell<f64>,
        #[property(get, set)]
        played: RefCell<bool>,
        #[property(get, set)]
        unplayed_item_count: RefCell<u32>,
        #[property(get, set)]
        is_favorite: RefCell<bool>,
        #[property(get, set)]
        is_resume: RefCell<bool>,
        #[property(get, set)]
        item_type: RefCell<String>,
        #[property(get, set)]
        production_year: RefCell<u32>,
        #[property(get, set, nullable)]
        parent_thumb_item_id: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        parent_backdrop_item_id: RefCell<Option<String>>,
        #[property(get, set)]
        poster: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        image_tags: RefCell<Option<crate::ui::provider::image_tags::ImageTags>>,
        #[property(get, set, nullable)]
        role: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        artists: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        album_id: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        rating: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        primary_image_item_id: RefCell<Option<String>>,
        #[property(get, set)]
        run_time_ticks: RefCell<u64>,
        #[property(get, set, nullable)]
        collection_type: RefCell<Option<String>>,
        #[property(name = "albumartist-name", get, set, type = String, member = name)]
        #[property(name = "albumartist-id", get, set, type = String, member = id)]
        album_artist: RefCell<AlbumArtist>,
        #[property(get, set, nullable)]
        program_name: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        program_start_time: RefCell<Option<DateTime>>,
        #[property(get, set, nullable)]
        program_end_time: RefCell<Option<DateTime>>,
        #[property(get, set, nullable)]
        premiere_date: RefCell<Option<DateTime>>,
        #[property(get, set, nullable)]
        status: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        end_date: RefCell<Option<DateTime>>,
        #[property(get, set, nullable)]
        overview: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        tagline: RefCell<Option<String>>,
        #[property(get, set)]
        playback_position_ticks: RefCell<u64>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for TuItem {}

    #[glib::object_subclass]
    impl ObjectSubclass for TuItem {
        const NAME: &'static str = "TuItem";
        type Type = super::TuItem;
    }

    impl TuItem {
        pub fn set_image_tags(&self, s: Option<crate::client::structs::ImageTags>) {
            let image_tags = crate::ui::provider::image_tags::ImageTags::new();
            if let Some(s) = s {
                image_tags.set_backdrop(s.backdrop.clone());
                image_tags.set_primary(s.primary.clone());
                image_tags.set_thumb(s.thumb.clone());
                image_tags.set_banner(s.banner.clone());
            }
            self.image_tags.replace(Some(image_tags));
        }
    }
}

glib::wrapper! {
    pub struct TuItem(ObjectSubclass<imp::TuItem>);
}

impl Default for TuItem {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl TuItem {
    pub fn from_simple(latest: &SimpleListItem, poster: Option<&str>) -> Self {
        let tu_item: TuItem = glib::object::Object::new();
        tu_item.set_id(latest.id.clone());
        tu_item.set_name(latest.name.clone());
        tu_item.set_item_type(latest.item_type.clone());
        if let Some(production_year) = latest.production_year {
            tu_item.set_production_year(production_year);
        }
        if let Some(index_number) = latest.index_number {
            tu_item.set_index_number(index_number);
        }
        if let Some(parent_index_number) = latest.parent_index_number {
            tu_item.set_parent_index_number(parent_index_number);
        }
        if let Some(userdata) = &latest.user_data {
            tu_item.set_played(userdata.played);
            if let Some(played_percentage) = userdata.played_percentage {
                tu_item.set_played_percentage(played_percentage);
            }
            if let Some(unplayed_item_count) = userdata.unplayed_item_count {
                tu_item.set_unplayed_item_count(unplayed_item_count);
            }
            if let Some(playback_position_ticks) = userdata.playback_position_ticks {
                tu_item.set_playback_position_ticks(playback_position_ticks);
            }
            tu_item.set_is_favorite(userdata.is_favorite.unwrap_or(false));
        }
        if let Some(poster) = poster {
            tu_item.set_poster(poster);
        }
        tu_item.imp().set_image_tags(latest.image_tags.clone());
        if let Some(parent_thumb_item_id) = &latest.parent_thumb_item_id {
            tu_item.set_parent_thumb_item_id(Some(parent_thumb_item_id.clone()));
        }
        if let Some(parent_backdrop_item_id) = &latest.parent_backdrop_item_id {
            tu_item.set_parent_backdrop_item_id(Some(parent_backdrop_item_id.clone()));
        }
        if let Some(series_name) = &latest.series_name {
            tu_item.set_series_name(series_name.clone());
        }
        if let Some(album_artist) = &latest.album_artists {
            tu_item.set_albumartist_name(
                album_artist
                    .first()
                    .as_ref()
                    .map(|s| s.name.as_str())
                    .unwrap_or_default()
                    .to_string(),
            );
            tu_item.set_albumartist_id(
                album_artist
                    .first()
                    .as_ref()
                    .map(|s| s.id.as_str())
                    .unwrap_or_default()
                    .to_string(),
            );
        }
        if let Some(role) = &latest.role {
            tu_item.set_role(Some(role.clone()));
        }
        if let Some(artists) = &latest.artists {
            let artist = artists.join(" , ");
            tu_item.set_artists(Some(artist));
        }
        if let Some(album_id) = &latest.album_id {
            tu_item.set_album_id(Some(album_id.clone()));
        }
        if let Some(run_time_ticks) = latest.run_time_ticks {
            tu_item.set_run_time_ticks(run_time_ticks);
        }
        if let Some(taglines) = &latest.taglines {
            tu_item.set_tagline(taglines.first().cloned());
        }
        if let Some(primary_image_item_id) = &latest.primary_image_item_id {
            tu_item.set_primary_image_item_id(Some(primary_image_item_id.clone()));
        }
        if let Some(rating) = &latest.community_rating {
            let rating = format!("{:.1}", rating);
            tu_item.set_rating(Some(rating));
        }
        if let Some(collection_type) = &latest.collection_type {
            tu_item.set_collection_type(Some(collection_type.clone()));
        }
        if let Some(current_program) = &latest.current_program {
            if let Some(program_name) = &current_program.name {
                tu_item.set_program_name(Some(program_name.clone()));
            }
            if let Some(start_time) = &current_program.start_date {
                tu_item.set_program_start_time(Some(&chrono_to_glib(start_time)));
            }
            if let Some(end_time) = &current_program.end_date {
                tu_item.set_program_end_time(Some(&chrono_to_glib(end_time)));
            }
        }
        if let Some(premiere_date) = &latest.premiere_date {
            tu_item.set_premiere_date(Some(&chrono_to_glib(premiere_date)));
        }
        if let Some(series_id) = &latest.series_id {
            tu_item.set_series_id(series_id.clone());
        }
        if let Some(status) = &latest.status {
            tu_item.set_status(Some(status.clone()));
        }
        if let Some(end_date) = &latest.end_date {
            tu_item.set_end_date(Some(&chrono_to_glib(end_date)));
        }
        if let Some(overview) = &latest.overview {
            tu_item.set_overview(Some(overview.clone()));
        }
        tu_item
    }

    pub fn activate<T>(&self, widget: &T, parentid: Option<String>)
    where
        T: gtk::prelude::WidgetExt + glib::clone::Downgrade,
    {
        let window = widget.root().and_downcast::<Window>().unwrap();

        if self.item_type() == "TvChannel" {
            self.tvchannel(window);
            return;
        }

        match self.item_type().as_str() {
            "Series" | "Movie" | "Video" | "MusicVideo" | "AdultVideo" => {
                let page = ItemPage::new(self);
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "Episode" => {
                let page = ItemPage::new(self);
                push_page_with_tag(
                    window,
                    page,
                    self.id(),
                    &self.series_name().unwrap_or_default(),
                );
            }
            "MusicAlbum" => {
                let page = AlbumPage::new(self.clone());
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "CollectionFolder" => {
                let page = ListPage::new(self.id(), self.collection_type().unwrap_or_default());
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "UserView" => {
                let page = ListPage::new(self.id(), "livetv".to_string());
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "Tag" | "Genre" | "MusicGenre" => {
                let page = SingleGrid::new();
                let id = self.id();
                let parent_id = parentid.clone();
                let list_type = self.item_type();
                page.connect_sort_changed_tokio(false, move |sort_by, sort_order| {
                    let id = id.clone();
                    let parent_id = parent_id.clone();
                    let list_type = list_type.clone();
                    async move {
                        EMBY_CLIENT
                            .get_inlist(parent_id, 0, &list_type, &id, &sort_order, &sort_by)
                            .await
                    }
                });
                let id = self.id();
                let parent_id = parentid.clone();
                let list_type = self.item_type();
                page.connect_end_edge_overshot_tokio(false, move |sort_by, sort_order, n_items| {
                    let id = id.clone();
                    let parent_id = parent_id.clone();
                    let list_type = list_type.clone();
                    async move {
                        EMBY_CLIENT
                            .get_inlist(parent_id, n_items, &list_type, &id, &sort_order, &sort_by)
                            .await
                    }
                });
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "Folder" => {
                let page = SingleGrid::new();
                page.set_list_type(ListType::Folder);
                let id = self.id();
                page.connect_sort_changed_tokio(false, move |sort_by, sort_order| {
                    let id = id.clone();
                    async move {
                        EMBY_CLIENT.get_folder_include(&id, &sort_by, &sort_order, 0).await
                    }
                });
                let id = self.id();
                page.connect_end_edge_overshot_tokio(false, move |sort_by, sort_order, n_items| {
                    let id = id.clone();
                    async move {
                        EMBY_CLIENT.get_folder_include(&id, &sort_by, &sort_order, n_items).await
                    }
                });
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            _ => {
                let page = OtherPage::new(self);
                push_page_with_tag(window, page, self.id(), &self.name());
            }
        }
    }

    fn tvchannel(&self, window: Window) {
        spawn(glib::clone!(
            #[strong(rename_to = item)]
            self,
            async move {
                toast!(window, gettext("Processing..."));
                let id = item.id();
                match spawn_tokio(async move { EMBY_CLIENT.get_live_playbackinfo(&id).await }).await
                {
                    Ok(playback) => {
                        let Some(ref url) = playback.media_sources[0].transcoding_url else {
                            toast!(window, gettext("No transcoding url found"));
                            return;
                        };
                        let back = Back {
                            tick: 0,
                            id: item.id(),
                            playsessionid: playback.play_session_id,
                            mediasourceid: playback.media_sources[0].id.clone(),
                            start_tick: glib::DateTime::now_local().unwrap().to_unix() as u64,
                        };
                        window.play_media(
                            url.to_string(),
                            None,
                            item,
                            Vec::new(),
                            Some(back),
                            None,
                            0.0,
                            None,
                        )
                    }
                    Err(e) => {
                        toast!(window, e.to_user_facing());
                    }
                }
            }
        ));
    }
}

fn chrono_to_glib(datetime: &chrono::DateTime<chrono::Utc>) -> DateTime {
    DateTime::from_iso8601(&datetime.to_rfc3339(), None).unwrap()
}

fn push_page_with_tag<T>(window: Window, page: T, tag: String, name: &str)
where
    T: NavigationPageExt,
{
    window.push_page(&page, &tag, name);
}
