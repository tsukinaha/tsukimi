use std::cell::RefCell;

use adw::prelude::*;
use gettextrs::gettext;
use glib::DateTime;
use gtk::{
    gio,
    glib::{
        self,
        subclass::prelude::*,
    },
};

#[allow(dead_code)] //FIXME: refactor with this
pub mod item_type {
    pub const MOVIE: &str = "Movie";
    pub const VIDEO: &str = "Video";
    pub const MUSIC_VIDEO: &str = "MusicVideo";
    pub const ADULT_VIDEO: &str = "AdultVideo";
    pub const TV_CHANNEL: &str = "TvChannel";
    pub const COLLECTION_FOLDER: &str = "CollectionFolder";
    pub const USER_VIEW: &str = "UserView";
    pub const SERIES: &str = "Series";
    pub const BOX_SET: &str = "BoxSet";
    pub const TAG: &str = "Tag";
    pub const GENRE: &str = "Genre";
    pub const MUSIC_GENRE: &str = "MusicGenre";
    pub const EPISODE: &str = "Episode";
    pub const VIEWS: &str = "Views";
    pub const MUSIC_ALBUM: &str = "MusicAlbum";
    pub const ACTOR: &str = "Actor";
    pub const PERSON: &str = "Person";
    pub const DIRECTOR: &str = "Director";
    pub const WRITER: &str = "Writer";
    pub const PRODUCER: &str = "Producer";
    pub const GUEST_STAR: &str = "GuestStar";
    pub const MUSIC_ARTIST: &str = "MusicArtist";
    pub const AUDIO: &str = "Audio";
    pub const PLAYLIST: &str = "Playlist";
    pub const FOLDER: &str = "Folder";
    pub const SEASON: &str = "Season";
}

pub use item_type::*;

use crate::{
    bing_song_model,
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::{
            SimpleListItem,
            SongWidgetView,
        },
    },
    ui::{
        GlobalToast,
        provider::{
            core_song::CoreSong,
            tu_item::item_type::{
                EPISODE,
                SERIES,
                TV_CHANNEL,
            },
        },
        widgets::{
            hortu_scrolled::UnifySize,
            item::ItemPage,
            list::ListPage,
            music_album::AlbumPage,
            other::OtherPage,
            single_grid::{
                SingleGrid,
                imp::ListType,
            },
            song_widget::SongWidget,
            utils::*,
            window::Window,
        },
    },
    utils::{
        CacheEvent,
        CachePolicy,
        fetch_with_cache,
        spawn,
        spawn_tokio,
    },
};

#[derive(Default, Clone)]
struct AlbumArtist {
    name: String,
    id: String,
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
#[repr(u32)]
#[enum_type(name = "PreferSize")]
pub enum PreferSize {
    #[default]
    Auto,
    Video,
    Post,
}

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
#[repr(u32)]
#[enum_type(name = "PreferPoster")]
pub enum PreferPoster {
    #[default]
    Auto,
    ParentPost,
    ParentVideo,
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
        #[property(get, set, nullable)]
        series_name: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        series_id: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        season_id: RefCell<Option<String>>,
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
        #[property(get, set, builder(PreferSize::default()))]
        prefer_size: RefCell<PreferSize>,
        #[property(get, set, builder(PreferPoster::default()))]
        prefer_poster: RefCell<PreferPoster>,
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
        #[property(get, set, nullable)]
        path: RefCell<Option<String>>,
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
                image_tags.set_backdrop(s.backdrop.to_owned());
                image_tags.set_primary(s.primary.to_owned());
                image_tags.set_thumb(s.thumb.to_owned());
                image_tags.set_banner(s.banner.to_owned());
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
        let item = latest.to_owned();
        tu_item.set_id(item.id);
        tu_item.set_name(item.name);
        tu_item.set_item_type(item.item_type);
        tu_item.set_production_year(item.production_year.unwrap_or_default());
        tu_item.set_index_number(item.index_number.unwrap_or_default());
        tu_item.set_parent_index_number(item.parent_index_number.unwrap_or_default());
        tu_item.set_path(item.path);

        if let Some(userdata) = &item.user_data {
            tu_item.set_played(userdata.played);
            tu_item.set_played_percentage(userdata.played_percentage.unwrap_or_default());
            tu_item.set_unplayed_item_count(userdata.unplayed_item_count.unwrap_or_default());
            tu_item
                .set_playback_position_ticks(userdata.playback_position_ticks.unwrap_or_default());
            tu_item.set_is_favorite(userdata.is_favorite.unwrap_or(false));
        }

        if let Some(poster) = poster {
            tu_item.set_poster(poster);
        }

        tu_item.imp().set_image_tags(item.image_tags);
        tu_item.set_parent_thumb_item_id(item.parent_thumb_item_id);
        tu_item.set_parent_backdrop_item_id(item.parent_backdrop_item_id);
        tu_item.set_series_name(item.series_name);

        if let Some(album_artist) = &item.album_artists {
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

        tu_item.set_role(item.role);
        tu_item.set_artists(item.artists.map(|artists| artists.join(" , ")));
        tu_item.set_album_id(item.album_id);
        tu_item.set_run_time_ticks(item.run_time_ticks.unwrap_or_default());
        tu_item.set_tagline(item.taglines.and_then(|taglines| taglines.first().cloned()));
        tu_item.set_primary_image_item_id(item.primary_image_item_id);
        tu_item.set_rating(item.community_rating.map(|rating| format!("{rating:.1}")));
        tu_item.set_collection_type(item.collection_type);

        if let Some(current_program) = item.current_program {
            tu_item.set_program_name(current_program.name);
            tu_item.set_program_start_time(current_program.start_date.as_ref().map(chrono_to_glib));
            tu_item.set_program_end_time(current_program.end_date.as_ref().map(chrono_to_glib));
        }

        tu_item.set_premiere_date(item.premiere_date.as_ref().map(chrono_to_glib));
        tu_item.set_series_id(item.series_id);
        tu_item.set_status(item.status);
        tu_item.set_end_date(item.end_date.as_ref().map(chrono_to_glib));
        tu_item.set_overview(item.overview);
        tu_item.set_season_id(item.season_id);

        tu_item
    }

    pub fn activate<T>(&self, widget: &T, parentid: Option<String>)
    where
        T: gtk::prelude::WidgetExt + glib::clone::Downgrade,
    {
        let window = widget.root().and_downcast::<Window>().unwrap();

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
            "MusicAlbum" | "Playlist" => {
                let page = AlbumPage::new(self.to_owned());
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "CollectionFolder" | "UserView" => {
                let page = ListPage::new(self.id(), self.collection_type().unwrap_or_default());
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "Tag" | "Genre" | "MusicGenre" => {
                let page = SingleGrid::new();
                page.set_unify_size(UnifySize::Majority);
                let id = self.id();
                let parent_id = parentid.to_owned();
                let list_type = self.item_type();
                page.connect_sort_changed_tokio(
                    false,
                    PreferPoster::Auto,
                    move |sort_by, sort_order, filters_list| {
                        let id = id.to_owned();
                        let parent_id = parent_id.to_owned();
                        let list_type = list_type.to_owned();
                        async move {
                            JELLYFIN_CLIENT
                                .get_inlist(
                                    parent_id,
                                    0,
                                    &list_type,
                                    &id,
                                    &sort_order,
                                    &sort_by,
                                    &filters_list,
                                )
                                .await
                        }
                    },
                );
                let id = self.id();
                let parent_id = parentid.to_owned();
                let list_type = self.item_type();
                page.connect_end_edge_overshot_tokio(
                    move |sort_by, sort_order, n_items, filters_list| {
                        let id = id.to_owned();
                        let parent_id = parent_id.to_owned();
                        let list_type = list_type.to_owned();
                        async move {
                            JELLYFIN_CLIENT
                                .get_inlist(
                                    parent_id,
                                    n_items,
                                    &list_type,
                                    &id,
                                    &sort_order,
                                    &sort_by,
                                    &filters_list,
                                )
                                .await
                        }
                    },
                );
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            "Folder" => {
                let page = SingleGrid::new();
                page.set_list_type(ListType::Folder);
                page.set_unify_size(UnifySize::Majority);
                let id = self.id();
                page.connect_sort_changed_tokio(
                    false,
                    PreferPoster::Auto,
                    move |sort_by, sort_order, filters_list| {
                        let id = id.to_owned();
                        async move {
                            JELLYFIN_CLIENT
                                .get_folder_include(&id, &sort_by, &sort_order, 0, &filters_list)
                                .await
                        }
                    },
                );
                let id = self.id();
                page.connect_end_edge_overshot_tokio(
                    move |sort_by, sort_order, n_items, filters_list| {
                        let id = id.to_owned();
                        async move {
                            JELLYFIN_CLIENT
                                .get_folder_include(
                                    &id,
                                    &sort_by,
                                    &sort_order,
                                    n_items,
                                    &filters_list,
                                )
                                .await
                        }
                    },
                );
                push_page_with_tag(window, page, self.id(), &self.name());
            }
            _ => {
                let page = OtherPage::new(self);
                push_page_with_tag(window, page, self.id(), &self.name());
            }
        }
    }

    pub fn play_tvchannel(&self, obj: &impl IsA<gtk::Widget>) {
        let binding = obj.root();
        let Some(window) = binding.and_downcast_ref::<Window>() else {
            return;
        };
        spawn(glib::clone!(
            #[strong(rename_to = item)]
            self,
            #[weak]
            window,
            async move {
                window.play_media(None, item, vec![], None, 0.0);
            }
        ));
    }

    pub fn play_single_audio(&self, obj: &impl IsA<gtk::Widget>) {
        let song_widget = SongWidget::new(self.to_owned(), SongWidgetView::MusicAlbumItem);
        let model = gio::ListStore::new::<CoreSong>();
        bing_song_model!(obj, model, song_widget.coresong());
    }

    pub async fn play_album(&self, obj: &impl IsA<gtk::Widget>) {
        let id = self.id();

        let mut events = fetch_with_cache(
            &format!("audio_{}", id),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_songs(&id).await },
        )
        .await;

        let Some(event) = events.recv().await else {
            return;
        };

        let songs = match event {
            CacheEvent::Data { data, .. } => data,
            CacheEvent::Error(e) => {
                obj.toast(e.to_user_facing());
                return;
            }
        };

        let song_widgets = songs
            .items
            .iter()
            .map(|song| {
                let item = TuItem::from_simple(song, None);
                let song_widget = SongWidget::new(item, SongWidgetView::MusicAlbumItem);
                song_widget.coresong()
            })
            .collect::<Vec<_>>();

        let Some(first) = song_widgets.first() else {
            return;
        };

        let model = gio::ListStore::new::<CoreSong>();
        model.extend_from_slice(&song_widgets);
        bing_song_model!(obj, model, first.to_owned());
    }

    pub async fn play_video(&self, obj: &impl IsA<gtk::Widget>) {
        self.direct_play_video_id(obj, self.to_owned(), Vec::new())
            .await;
    }

    pub async fn direct_play_video_id(
        &self, obj: &impl IsA<gtk::Widget>, video: TuItem, episode_list: Vec<TuItem>,
    ) {
        if let Some(window) = obj.root().and_downcast_ref::<Window>() {
            window.play_media(
                None,
                video,
                episode_list,
                None,
                self.playback_position_ticks() as f64 / 10_000_000.0,
            )
        }
    }

    pub async fn play_series(&self, obj: &impl IsA<gtk::Widget>) {
        let id = self.id();

        let nextup_list =
            match spawn_tokio(async move { JELLYFIN_CLIENT.get_shows_next_up(&id).await }).await {
                Ok(list) => list,
                Err(e) => {
                    obj.toast(e.to_user_facing());
                    return;
                }
            };

        let Some(nextup_item) = nextup_list.items.first() else {
            obj.toast(gettext("No next up video found"));
            return;
        };

        self.direct_play_video_id(
            obj,
            TuItem::from_simple(nextup_item, None),
            nextup_list
                .items
                .iter()
                .map(|item| TuItem::from_simple(item, None))
                .collect(),
        )
        .await;
    }

    pub fn fmt_period(&self) -> String {
        // '2022' '2022 - 2023' '2022 - Present' '2022 - Unknown'
        let production_year = self.production_year();

        if production_year == 0 {
            return String::new();
        }

        let Some(status) = self.status() else {
            return production_year.to_string();
        };

        if status.as_str() == "Continuing" {
            return format!("{production_year} - {}", gettext("Present"));
        }

        if status.as_str() != "Ended" {
            return production_year.to_string();
        }

        let Some(end_date) = self.end_date() else {
            return format!("{production_year} - {}", gettext("Unknown"));
        };

        let end_year = end_date.year();

        if end_year != production_year as i32 {
            format!("{production_year} - {end_year}")
        } else {
            format!("{end_year}")
        }
    }

    pub fn fmt_production_year(&self) -> String {
        let production_year = self.production_year();

        if production_year != 0 {
            production_year.to_string()
        } else {
            gettext("Unknown")
        }
    }

    pub fn fmt_tv_name(&self) -> String {
        let Some(program_name) = self.program_name() else {
            return self.name();
        };

        format!("{} - {program_name})", self.name())
    }

    pub fn fmt_episode_detail(&self) -> String {
        format!(
            "S{}E{}: {}",
            self.parent_index_number(),
            self.index_number(),
            self.name()
        )
    }

    pub fn fmt_tv_progress_and_start_end_time(&self) -> (f64, String) {
        fn default() -> (f64, String) {
            (0.0, gettext("Unknown").to_string())
        }

        let Some(Ok(program_start_time)) = self.program_start_time().map(|t| t.to_local()) else {
            return default();
        };

        let Some(Ok(program_end_time)) = self.program_end_time().map(|t| t.to_local()) else {
            return default();
        };

        let Ok(now) = glib::DateTime::now_local() else {
            return default();
        };

        let progress = ((now.to_unix() - program_start_time.to_unix()) as f64
            / (program_end_time.to_unix() - program_start_time.to_unix()) as f64)
            * 100.0;

        let start_end_time = format!(
            "{} - {}",
            program_start_time
                .format("%H:%M")
                .unwrap_or_else(|_| gettext("Unknown").into()),
            program_end_time
                .format("%H:%M")
                .unwrap_or_else(|_| gettext("Unknown").into())
        );

        (progress, start_end_time)
    }

    pub fn fmt_season_premiere_date(&self) -> String {
        self.premiere_date()
            .and_then(|premiere_date| premiere_date.format("%Y-%m-%d").ok())
            .unwrap_or_default()
            .into()
    }

    pub fn fmt_title(&self) -> String {
        let title = match self.item_type().as_str() {
            TV_CHANNEL => self.fmt_tv_name(),
            EPISODE if let Some(series_name) = self.series_name() => series_name,
            _ => self.name(),
        };

        if self.has_unplayed_item() && self.unplayed_item_count() > 0 {
            format!("{} ({})", title, self.unplayed_item_count())
        } else {
            title
        }
    }

    pub fn fmt_subtitle(&self) -> String {
        match self.item_type().as_str() {
            TV_CHANNEL => self.fmt_tv_progress_and_start_end_time().1,
            MOVIE => self.fmt_production_year(),
            EPISODE if self.series_name().is_some() => self.fmt_episode_detail(),
            SEASON => self.fmt_season_premiere_date(),
            SERIES => self.fmt_period(),
            ACTOR | PERSON | DIRECTOR | WRITER | PRODUCER | GUEST_STAR => {
                self.role().unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    pub fn fmt_percentage(&self) -> Option<f64> {
        match self.item_type().as_str() {
            TV_CHANNEL => Some(self.fmt_tv_progress_and_start_end_time().0),
            MOVIE if self.is_resume() => Some(self.played_percentage()),
            EPISODE => Some(self.played_percentage()),
            _ => None,
        }
    }

    pub fn has_unplayed_item(&self) -> bool {
        match self.item_type().as_str() {
            SERIES | SEASON => true,
            MOVIE if !self.is_resume() => true,
            EPISODE if !self.is_resume() => true,
            _ => false,
        }
    }

    pub fn has_played_mark(&self) -> bool {
        self.has_unplayed_item() && self.played()
    }

    pub fn has_direct_play_mark(&self) -> bool {
        match self.item_type().as_str() {
            MOVIE if self.is_resume() => true,
            EPISODE if self.is_resume() => true,
            _ => false,
        }
    }

    pub fn has_folder_mark(&self) -> bool {
        matches!(self.item_type().as_str(), FOLDER)
    }

    pub fn size_hint(&self) -> (i32, i32) {
        match self.prefer_size() {
            PreferSize::Video => return TU_ITEM_VIDEO_SIZE,
            PreferSize::Post => return TU_ITEM_POST_SIZE,
            _ => (),
        }

        match self.item_type().as_str() {
            MOVIE | SERIES | BOX_SET | ACTOR | PERSON | DIRECTOR | WRITER | PRODUCER
            | GUEST_STAR | SEASON => TU_ITEM_POST_SIZE,
            VIDEO | MUSIC_VIDEO | ADULT_VIDEO | TV_CHANNEL | COLLECTION_FOLDER | EPISODE
            | USER_VIEW => TU_ITEM_VIDEO_SIZE,
            _ => TU_ITEM_SQUARE_SIZE,
        }
    }

    pub fn list_item_title(&self) -> Option<String> {
        let name = self.name();

        if name.is_empty() {
            return None;
        }

        let title = match self.item_type().as_str() {
            TV_CHANNEL | SEASON | BOX_SET | MUSIC_ALBUM | GENRE | TAG | FOLDER => name,
            EPISODE if self.series_name().is_some() => self.fmt_subtitle(),
            MOVIE if self.is_resume() => self.fmt_title(),
            PERSON | DIRECTOR | WRITER | PRODUCER | GUEST_STAR | ACTOR => {
                if let Some(role) = self.role() {
                    format!("{name} / {role}")
                } else {
                    name
                }
            }
            _ => return None,
        };

        Some(title)
    }

    pub fn fmt_rating(&self) -> Option<String> {
        self.rating()
    }

    pub fn need_animated_picture(&self) -> bool {
        matches!(self.item_type().as_str(), COLLECTION_FOLDER)
    }

    pub fn can_direct_play(&self) -> bool {
        matches!(self.item_type().as_str(), MOVIE | EPISODE) && self.is_resume()
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
