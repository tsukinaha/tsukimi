use chrono::{DateTime, Utc};
use derive_builder::Builder;
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
    #[serde(rename = "Index")]
    pub index: u64,
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
    #[serde(rename = "PartCount")]
    pub part_count: Option<u32>,
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
    #[serde(rename = "Status")]
    pub status: Option<String>,
    #[serde(rename = "EndDate")]
    pub end_date: Option<DateTime<Utc>>,
    #[serde(rename = "PremiereDate")]
    pub premiere_date: Option<DateTime<Utc>>,
    #[serde(rename = "Taglines")]
    pub taglines: Option<Vec<String>>,
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
    pub items: Vec<SimpleListItem>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Items {
    #[serde(rename = "Items")]
    pub items: Vec<Item>,
    #[serde(rename = "TotalRecordCount")]
    pub total_record_count: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ExternalIdInfo {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "UrlFormatString")]
    pub url_format_string: String,
    #[serde(rename = "IsSupportedAsIdentifier")]
    pub is_supported_as_identifier: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RemoteSearchInfo {
    #[serde(rename = "ItemId")]
    pub item_id: String,
    #[serde(rename = "SearchInfo")]
    pub search_info: SearchInfo,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SearchInfo {
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Year")]
    pub year: Option<u32>,
    #[serde(rename = "ProviderIds")]
    pub provider_ids: Vec<SearchProviderId>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SearchProviderId {
    #[serde(rename = "MusicBrainzAlbum")]
    pub music_brainz_album: Option<String>,
    #[serde(rename = "MusicBrainzAlbumArtist")]
    pub music_brainz_album_artist: Option<String>,
    #[serde(rename = "MusicBrainzReleaseGroup")]
    pub music_brainz_release_group: Option<String>,
    #[serde(rename = "Tmdb")]
    pub tmdb: Option<String>,
    #[serde(rename = "Tvdb")]
    pub tvdb: Option<String>,
    #[serde(rename = "IMDB")]
    pub imdb: Option<String>,
    #[serde(rename = "Zap2It")]
    pub zap2it: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RemoteSearchResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ProductionYear")]
    pub production_year: Option<u32>,
    #[serde(rename = "ImageUrl")]
    pub image_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ServerInfo {
    #[serde(rename = "ServerName")]
    pub server_name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "LocalAddress")]
    pub local_address: String,
    #[serde(rename = "WanAddress")]
    pub wan_address: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ActivityLog {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Date")]
    pub date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ScheduledTask {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "LastExecutionResult")]
    pub last_execution_result: Option<LastExecutionResult>,
    #[serde(rename = "Description")]
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LastExecutionResult {
    #[serde(rename = "StartTimeUtc")]
    pub start_time_utc: DateTime<Utc>,
    #[serde(rename = "EndTimeUtc")]
    pub end_time_utc: DateTime<Utc>,
    #[serde(rename = "Status")]
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ActivityLogs {
    #[serde(rename = "Items")]
    pub item: Vec<ActivityLog>,
}

#[derive(Deserialize, Debug, Clone, Builder)]
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

use crate::ui::widgets::{single_grid::SingleGrid, window::Window};
use adw::prelude::*;
use gtk::glib;

use super::client::EMBY_CLIENT;

impl SGTitem {
    pub fn activate<T>(&self, widget: &T, list_type: String)
    where
        T: gtk::prelude::WidgetExt + glib::clone::Downgrade,
    {
        let page = SingleGrid::new();
        let id = self.id.to_string();
        let list_type_clone = list_type.clone();
        page.connect_sort_changed_tokio(false, move |sort_by, sort_order| {
            let id = id.clone();
            let list_type_clone = list_type_clone.clone();
            async move {
                EMBY_CLIENT
                    .get_inlist(None, 0, &list_type_clone, &id, &sort_order, &sort_by)
                    .await
            }
        });
        let id = self.id.to_string();
        let list_type = list_type.clone();
        page.connect_end_edge_overshot_tokio(false, move |sort_by, sort_order, n_items| {
            let id = id.clone();
            let list_type = list_type.clone();
            async move {
                EMBY_CLIENT
                    .get_inlist(None, n_items, &list_type, &id, &sort_order, &sort_by)
                    .await
            }
        });
        page.emit_by_name::<()>("sort-changed", &[]);
        push_page_with_tag(widget, page, self.name.clone());
    }
}

pub fn push_page_with_tag<T, R>(widget: &R, page: T, tag: String)
where
    T: NavigationPageExt,
    R: gtk::prelude::WidgetExt + glib::clone::Downgrade,
{
    page.set_tag(Some(&tag));
    let window = widget.root().and_downcast::<Window>().unwrap();
    window.push_page(&page);
}
