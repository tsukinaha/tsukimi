use chrono::{DateTime, Utc};
use gettextrs::gettext;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct SearchResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub result_type: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "UserData")]
    pub user_data: Option<UserData>,
    #[serde(rename = "ProductionYear")]
    pub production_year: Option<i16>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthenticateResponse {
    #[serde(rename = "Policy")]
    pub policy: Policy,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Policy {
    #[serde(rename = "IsAdministrator")]
    pub is_administrator: bool,
}

// single item
#[derive(Serialize, Deserialize, Clone)]
pub struct SeriesInfo {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Overview")]
    pub overview: Option<String>,
    #[serde(rename = "IndexNumber")]
    pub index_number: Option<u32>,
    #[serde(rename = "ParentIndexNumber")]
    pub parent_index_number: Option<u32>,
    #[serde(rename = "UserData")]
    pub user_data: Option<UserData>,
}

// media info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaStream {
    #[serde(rename = "DisplayTitle")]
    pub display_title: Option<String>,
    #[serde(rename = "Type")]
    pub stream_type: String,
    #[serde(rename = "DeliveryUrl")]
    pub delivery_url: Option<String>,
    #[serde(rename = "IsExternal")]
    pub is_external: bool,
    #[serde(rename = "Title")]
    pub title: Option<String>,
    #[serde(rename = "DisplayLanguage")]
    pub display_language: Option<String>,
    #[serde(rename = "Codec")]
    pub codec: Option<String>,
    #[serde(rename = "BitRate")]
    pub bit_rate: Option<u64>,
    #[serde(rename = "BitDepth")]
    pub bit_depth: Option<u64>,
    #[serde(rename = "AverageFrameRate")]
    pub average_frame_rate: Option<f64>,
    #[serde(rename = "Height")]
    pub height: Option<u64>,
    #[serde(rename = "Width")]
    pub width: Option<u64>,
    #[serde(rename = "PixelFormat")]
    pub pixel_format: Option<String>,
    #[serde(rename = "ColorSpace")]
    pub color_space: Option<String>,
    #[serde(rename = "SampleRate")]
    pub sample_rate: Option<u64>,
    #[serde(rename = "Channels")]
    pub channels: Option<u64>,
    #[serde(rename = "ChannelLayout")]
    pub channel_layout: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaSource {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "Path")]
    pub path: Option<String>,
    #[serde(rename = "Container")]
    pub container: String,
    #[serde(rename = "DirectStreamUrl")]
    pub direct_stream_url: Option<String>,
    #[serde(rename = "MediaStreams")]
    pub media_streams: Vec<MediaStream>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Media {
    #[serde(rename = "MediaSources")]
    pub media_sources: Vec<MediaSource>,
    #[serde(rename = "PlaySessionId")]
    pub play_session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveMedia {
    #[serde(rename = "MediaSources")]
    pub media_sources: Vec<LiveMediaSource>,
    #[serde(rename = "PlaySessionId")]
    pub play_session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveMediaSource {
    #[serde(rename = "TranscodingUrl")]
    pub transcoding_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Item {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "DateCreated")]
    pub date_created: Option<DateTime<Utc>>,
    #[serde(rename = "SeriesId")]
    pub series_id: Option<String>,
    #[serde(rename = "SeriesName")]
    pub series_name: Option<String>,
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(rename = "ParentIndexNumber")]
    pub parent_index_number: Option<u32>,
    #[serde(rename = "IndexNumber")]
    pub index_number: Option<u32>,
    #[serde(rename = "ProductionYear")]
    pub production_year: Option<u32>,
    #[serde(rename = "ExternalUrls")]
    pub external_urls: Option<Vec<Urls>>,
    #[serde(rename = "Overview")]
    pub overview: Option<String>,
    #[serde(rename = "People")]
    pub people: Option<Vec<SimpleListItem>>,
    #[serde(rename = "Studios")]
    pub studios: Option<Vec<SGTitem>>,
    #[serde(rename = "GenreItems")]
    pub genres: Option<Vec<SGTitem>>,
    #[serde(rename = "TagItems")]
    pub tags: Option<Vec<SGTitem>>,
    #[serde(rename = "UserData")]
    pub user_data: Option<UserData>,
    #[serde(rename = "CommunityRating")]
    pub community_rating: Option<f64>,
    #[serde(rename = "OfficialRating")]
    pub official_rating: Option<String>,
    #[serde(rename = "RunTimeTicks")]
    pub run_time_ticks: Option<u64>,
    #[serde(rename = "Taglines")]
    pub taglines: Option<Vec<String>>,
    #[serde(rename = "BackdropImageTags")]
    pub backdrop_image_tags: Option<Vec<String>>,
    #[serde(rename = "AlbumArtist")]
    pub album_artist: Option<String>,
    #[serde(rename = "MediaSources")]
    pub media_sources: Option<Vec<MediaSource>>,
    #[serde(rename = "PlaySessionId")]
    pub play_session_id: Option<String>,
    #[serde(rename = "OriginalTitle")]
    pub original_title: Option<String>,
    #[serde(rename = "SortName")]
    pub sort_name: Option<String>,
    #[serde(rename = "ProviderIds")]
    pub provider_ids: Option<ProviderIds>,
    #[serde(rename = "Path")]
    pub path: Option<String>,
    #[serde(rename = "Album")]
    pub album: Option<String>,
    #[serde(rename = "Artists")]
    pub artists: Option<Vec<String>>,
    #[serde(rename = "LockData")]
    pub lock_data: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ProviderIds {
    #[serde(rename = "Tmdb")]
    pub tmdb: Option<String>,
    #[serde(rename = "Imdb")]
    pub imdb: Option<String>,
    #[serde(rename = "Tvdb")]
    pub tvdb: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct People {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Role")]
    pub role: Option<String>,
    #[serde(rename = "Type")]
    pub people_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ImageItem {
    #[serde(rename = "Filename")]
    pub filename: Option<String>,
    #[serde(rename = "Height")]
    pub height: Option<u32>,
    #[serde(rename = "Width")]
    pub width: Option<u32>,
    #[serde(rename = "ImageType")]
    pub image_type: String,
    #[serde(rename = "Size")]
    pub size: Option<u64>,
    #[serde(rename = "ImageIndex")]
    pub image_index: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SGTitem {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Urls {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Url")]
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Resume {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub resume_type: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "SeriesId")]
    pub series_id: Option<String>,
    #[serde(rename = "IndexNumber")]
    pub index_number: Option<u32>,
    #[serde(rename = "ParentIndexNumber")]
    pub parent_index_number: Option<u32>,
    #[serde(rename = "ParentThumbItemId")]
    pub parent_thumb_item_id: Option<String>,
    #[serde(rename = "SeriesName")]
    pub series_name: Option<String>,
    #[serde(rename = "UserData")]
    pub user_data: Option<UserData>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserData {
    #[serde(rename = "PlayedPercentage")]
    pub played_percentage: Option<f64>,
    #[serde(rename = "PlaybackPositionTicks")]
    pub playback_position_ticks: Option<u64>,
    #[serde(rename = "Played")]
    pub played: bool,
    #[serde(rename = "UnplayedItemCount")]
    pub unplayed_item_count: Option<u32>,
    #[serde(rename = "IsFavorite")]
    pub is_favorite: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct View {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "CollectionType")]
    pub collection_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SimpleListItem {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Type")]
    pub latest_type: String,
    #[serde(rename = "UserData")]
    pub user_data: Option<UserData>,
    #[serde(rename = "ProductionYear")]
    pub production_year: Option<u32>,
    #[serde(rename = "IndexNumber")]
    pub index_number: Option<u32>,
    #[serde(rename = "ParentIndexNumber")]
    pub parent_index_number: Option<u32>,
    #[serde(rename = "SeriesName")]
    pub series_name: Option<String>,
    #[serde(rename = "ParentBackdropItemId")]
    pub parent_backdrop_item_id: Option<String>,
    #[serde(rename = "ParentThumbItemId")]
    pub parent_thumb_item_id: Option<String>,
    #[serde(rename = "PlayedPercentage")]
    pub played_percentage: Option<f64>,
    #[serde(rename = "ImageTags")]
    pub image_tags: Option<ImageTags>,
    #[serde(rename = "SeriesId")]
    pub series_id: Option<String>,
    #[serde(rename = "AlbumArtists")]
    pub album_artists: Option<Vec<View>>,
    #[serde(rename = "Artists")]
    pub artists: Option<Vec<String>>,
    #[serde(rename = "AlbumId")]
    pub album_id: Option<String>,
    #[serde(rename = "Role")]
    pub role: Option<String>,
    #[serde(rename = "RunTimeTicks")]
    pub run_time_ticks: Option<u64>,
    #[serde(rename = "PrimaryImageItemId")]
    pub primary_image_item_id: Option<String>,
    #[serde(rename = "BackdropImageTags")]
    pub backdrop_image_tags: Option<Vec<String>>,
    #[serde(rename = "CommunityRating")]
    pub community_rating: Option<f32>,
    #[serde(rename = "CollectionType")]
    pub collection_type: Option<String>,
    #[serde(rename = "Overview")]
    pub overview: Option<String>,
    #[serde(rename = "CurrentProgram")]
    pub current_program: Option<CurrentProgram>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CurrentProgram {
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "EndDate")]
    pub end_date: Option<DateTime<Utc>>,
    #[serde(rename = "StartDate")]
    pub start_date: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageTags {
    #[serde(rename = "Primary")]
    pub primary: Option<String>,
    #[serde(rename = "Thumb")]
    pub thumb: Option<String>,
    #[serde(rename = "Banner")]
    pub banner: Option<String>,
    #[serde(rename = "Backdrop")]
    pub backdrop: Option<String>,
    #[serde(rename = "Logo")]
    pub logo: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct List {
    #[serde(rename = "TotalRecordCount")]
    pub total_record_count: u32,
    #[serde(rename = "Items")]
    pub items: Vec<SimpleListItem>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SerInList {
    #[serde(rename = "Items")]
    pub items: Vec<SeriesInfo>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Items {
    #[serde(rename = "Items")]
    pub items: Vec<Item>,
    #[serde(rename = "TotalRecordCount")]
    pub total_record_count: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Back {
    pub id: String,
    pub playsessionid: Option<String>,
    pub mediasourceid: String,
    pub tick: u64,
}

#[derive(Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "User")]
    pub user: User,
    #[serde(rename = "AccessToken")]
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct User {
    #[serde(rename = "Id")]
    pub id: String,
}

use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::ui::widgets::singlelist::SingleListPage;
use crate::ui::widgets::window::Window;
use crate::utils::spawn_tokio;
use crate::{
    toast,
    ui::{
        provider::tu_item::TuItem,
        widgets::{
            actor::ActorPage, boxset::BoxSetPage, item::ItemPage, list::ListPage,
            music_album::AlbumPage,
        },
    },
    utils::spawn,
};
use adw::prelude::*;
use gtk::glib;

impl SGTitem {
    pub fn activate<T>(&self, widget: &T, list_type: String)
    where
        T: gtk::prelude::WidgetExt + glib::clone::Downgrade,
    {
        let window = widget.root().and_downcast::<Window>().unwrap();
        let page = SingleListPage::new(self.id.to_string(),"".to_string(),
        &list_type,
        None,
        true,);
        push_page_with_tag(window, page, self.name.clone());
    }

}

impl SimpleListItem {
    pub fn activate<T>(&self, widget: &T)
    where
        T: gtk::prelude::WidgetExt + glib::clone::Downgrade,
    {
        let window = widget.root().and_downcast::<Window>().unwrap();

        if self.latest_type == "TvChannel" {
            self.tvchannel(window);
            return;
        }

        match self.latest_type.as_str() {
            "Series" | "Movie" => {
                let page = ItemPage::new(self.id.clone(), self.id.clone(), self.name.clone());
                push_page_with_tag(window, page, self.name.clone());
            }
            "Episode" => {
                let page = ItemPage::new(
                    self.series_id.clone().unwrap(),
                    self.id.clone(),
                    self.name.clone(),
                );
                push_page_with_tag(window, page, self.series_name.clone().unwrap_or_default());
            }
            "MusicAlbum" => {
                let page = AlbumPage::new(TuItem::from_simple(self, None));
                push_page_with_tag(window, page, self.name.clone());
            }
            "Actor" | "Director" | "Person" | "Writer" => {
                let page = ActorPage::new(&self.id);
                push_page_with_tag(window, page, self.name.clone());
            }
            "BoxSet" => {
                let page = BoxSetPage::new(&self.id);
                push_page_with_tag(window, page, self.name.clone());
            }
            "CollectionFolder" | "UserView" => {
                let page = ListPage::new(
                    self.id.clone(),
                    self.collection_type.clone().unwrap_or_default(),
                );
                push_page_with_tag(window, page, self.name.clone());
            }
            _ => toast!(window, gettext("Not Supported Type")),
        }
    }

    fn tvchannel(&self, window: Window) {
        spawn(glib::clone!(
            #[strong(rename_to = item)]
            self,
            async move {
                toast!(window, gettext("Processing..."));
                match spawn_tokio(async move { EMBY_CLIENT.get_live_playbackinfo(&item.id).await })
                    .await
                {
                    Ok(playback) => {
                        let Some(ref url) = playback.media_sources[0].transcoding_url else {
                            toast!(window, gettext("No transcoding url found"));
                            return;
                        };
                        window.play_media(url.to_string(), None, Some(item.name), None, None, 0.0)
                    }
                    Err(e) => {
                        toast!(window, e.to_user_facing());
                    }
                }
            }
        ));
    }
}

fn push_page_with_tag<T>(window: Window, page: T, tag: String)
where
    T: NavigationPageExt,
{
    page.set_tag(Some(&tag));
    window.push_page(&page);
}
