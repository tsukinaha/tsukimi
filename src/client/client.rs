use std::sync::Mutex;

use anyhow::{anyhow, Result};

use reqwest::{header::HeaderValue, Method, RequestBuilder, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, warn};
use url::Url;
use uuid::Uuid;

use crate::{
    config::{proxy::ReqClient, Account, APP_VERSION},
    ui::{models::emby_cache_path, widgets::single_grid::imp::ListType},
    utils::{spawn, spawn_tokio},
};

use once_cell::sync::Lazy;

use super::structs::{
    ActivityLogs, AuthenticateResponse, Back, ExternalIdInfo, ImageItem, Item, List, LiveMedia,
    LoginResponse, Media, PublicServerInfo, RemoteSearchInfo, RemoteSearchResult, ScheduledTask,
    SerInList, ServerInfo, SimpleListItem,
};

pub static EMBY_CLIENT: Lazy<EmbyClient> = Lazy::new(EmbyClient::default);
pub static DEVICE_ID: Lazy<String> = Lazy::new(|| Uuid::new_v4().to_string());
static PROFILE: &str = include_str!("stream_profile.json");
static LIVEPROFILE: &str = include_str!("test.json");
static CLIENT_ID: Lazy<String> = Lazy::new(|| "Tsukimi".to_string());
static DEVICE_NAME: Lazy<String> = Lazy::new(|| {
    hostname::get()
        .unwrap_or("Unknown".into())
        .to_string_lossy()
        .to_string()
});

#[derive(PartialEq)]
pub enum BackType {
    Start,
    Stop,
    Back,
}

pub struct EmbyClient {
    pub url: Mutex<Option<Url>>,
    pub client: reqwest::Client,
    pub headers: Mutex<reqwest::header::HeaderMap>,
    pub user_id: Mutex<String>,
    pub user_name: Mutex<String>,
    pub user_password: Mutex<String>,
    pub user_access_token: Mutex<String>,
    pub server_name: Mutex<String>,
}

impl EmbyClient {
    pub fn default() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", HeaderValue::from_static("Emby"));
        headers.insert("X-Emby-Client", HeaderValue::from_static(&CLIENT_ID));
        headers.insert(
            "X-Emby-Device-Name",
            HeaderValue::from_str(&DEVICE_NAME).unwrap(),
        );
        headers.insert(
            "X-Emby-Device-Id",
            HeaderValue::from_str(&DEVICE_ID).unwrap(),
        );
        headers.insert(
            "X-Emby-Client-Version",
            HeaderValue::from_static(APP_VERSION),
        );
        headers.insert("X-Emby-Language", HeaderValue::from_static("zh-cn"));
        Self {
            url: Mutex::new(None),
            client: ReqClient::build(),
            headers: Mutex::new(headers),
            user_id: Mutex::new(String::new()),
            user_name: Mutex::new(String::new()),
            user_password: Mutex::new(String::new()),
            user_access_token: Mutex::new(String::new()),
            server_name: Mutex::new(String::new()),
        }
    }

    pub fn init(&self, account: &Account) -> Result<(), Box<dyn std::error::Error>> {
        self.header_change_url(&account.server, &account.port)?;
        self.header_change_token(&account.access_token)?;
        self.set_user_id(&account.user_id)?;
        self.set_user_name(&account.username)?;
        self.set_user_password(&account.password)?;
        self.set_user_access_token(&account.access_token)?;
        self.set_server_name(&account.servername)?;
        crate::ui::provider::set_admin(false);
        spawn(async move {
            spawn_tokio(async move {
                match EMBY_CLIENT.authenticate_admin().await {
                    Ok(r) => {
                        if r.policy.is_administrator {
                            crate::ui::provider::set_admin(true);
                        }
                    }
                    Err(e) => warn!("Failed to authenticate as admin: {}", e),
                }
            })
            .await;
        });
        Ok(())
    }

    pub fn header_change_token(&self, token: &str) -> Result<()> {
        let mut headers = self
            .headers
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on headers"))?;
        headers.insert("X-Emby-Token", HeaderValue::from_str(token)?);
        Ok(())
    }

    pub fn header_change_url(&self, url: &str, port: &str) -> Result<()> {
        let mut url = Url::parse(url)?;
        url.set_port(Some(port.parse::<u16>().unwrap_or_default()))
            .map_err(|_| anyhow!("Failed to set port"))?;
        let mut url_lock = self
            .url
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on URL"))?;
        *url_lock = Some(url.join("emby/")?);
        Ok(())
    }

    pub fn set_user_id(&self, user_id: &str) -> Result<()> {
        let mut user_id_lock = self
            .user_id
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on user_id"))?;
        *user_id_lock = user_id.to_string();
        Ok(())
    }

    pub fn set_user_name(&self, user_name: &str) -> Result<()> {
        let mut user_name_lock = self
            .user_name
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on user_name"))?;
        *user_name_lock = user_name.to_string();
        Ok(())
    }

    pub fn set_user_password(&self, user_password: &str) -> Result<()> {
        let mut user_password_lock = self
            .user_password
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on user_password"))?;
        *user_password_lock = user_password.to_string();
        Ok(())
    }

    pub fn set_user_access_token(&self, user_access_token: &str) -> Result<()> {
        let mut user_access_token_lock = self
            .user_access_token
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on user_access_token"))?;
        *user_access_token_lock = user_access_token.to_string();
        Ok(())
    }

    pub fn set_server_name(&self, server_name: &str) -> Result<()> {
        let mut server_name_lock = self
            .server_name
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on server_name"))?;
        *server_name_lock = server_name.to_string();
        Ok(())
    }

    pub fn get_url_and_headers(&self) -> Result<(Url, reqwest::header::HeaderMap)> {
        let url = self
            .url
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on URL"))?
            .as_ref()
            .ok_or_else(|| anyhow!("URL is not set"))?
            .clone();
        let headers = self
            .headers
            .lock()
            .map_err(|_| anyhow!("Failed to acquire lock on headers"))?
            .clone();
        Ok((url, headers))
    }

    pub async fn request<T>(&self, path: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let request = self.prepare_request(Method::GET, path, params)?;
        let res = self.send_request(request).await?.error_for_status()?;

        let json = res.json().await?;
        Ok(json)
    }

    pub async fn request_picture(&self, path: &str, params: &[(&str, &str)]) -> Result<Response> {
        let request = self.prepare_request(Method::GET, path, params)?;
        let res = request.send().await?;
        Ok(res)
    }

    pub async fn post<B>(&self, path: &str, params: &[(&str, &str)], body: B) -> Result<Response>
    where
        B: Serialize,
    {
        let request = self
            .prepare_request(Method::POST, path, params)?
            .json(&body);
        let res = self.send_request(request).await?;
        Ok(res)
    }

    pub async fn post_json<B, T>(
        &self,
        path: &str,
        params: &[(&str, &str)],
        body: B,
    ) -> Result<T, anyhow::Error>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let response = self.post(path, params, body).await?.error_for_status()?;
        let parsed = response.json::<T>().await?;
        Ok(parsed)
    }

    fn prepare_request(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<RequestBuilder> {
        let (mut url, headers) = self.get_url_and_headers()?;
        url = url.join(path)?;
        self.add_params_to_url(&mut url, params);
        Ok(self.client.request(method, url).headers(headers))
    }

    async fn send_request(&self, request: RequestBuilder) -> Result<Response> {
        let res = request.send().await?;
        Ok(res)
    }

    pub async fn authenticate_admin(&self) -> Result<AuthenticateResponse> {
        let path = format!("Users/{}", self.user_id());
        let res = self.request(&path, &[]).await?;
        Ok(res)
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResponse> {
        let body = json!({
            "Username": username,
            "Pw": password
        });
        self.post_json("Users/authenticatebyname", &[], body).await
    }

    pub fn add_params_to_url(&self, url: &mut Url, params: &[(&str, &str)]) {
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, value);
        }
        info!("Request URL: {}", url);
    }

    pub async fn search(&self, query: &str, filter: &[&str], start_index: &str) -> Result<List> {
        let filter_str = filter.join(",");
        let path = format!("Users/{}/Items", self.user_id());
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("IncludeItemTypes", &filter_str),
            ("IncludeSearchTypes", &filter_str),
            ("StartIndex", start_index),
            ("SortBy", "SortName"),
            ("SortOrder", "Ascending"),
            ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
            ("ImageTypeLimit", "1"),
            ("Recursive", "true"),
            ("SearchTerm", query),
            ("GroupProgramsBySeries", "true"),
            ("Limit", "50"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_episodes(&self, id: &str, season_id: &str) -> Result<SerInList> {
        let path = format!("Shows/{}/Episodes", id);
        let params = [
            (
                "Fields",
                "Overview,PrimaryImageAspectRatio,PremiereDate,ProductionYear,SyncStatus",
            ),
            ("ImageTypeLimit", "1"),
            ("SeasonId", season_id),
            ("UserId", &self.user_id()),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_item_info(&self, id: &str) -> Result<Item> {
        let path = format!("Users/{}/Items/{}", self.user_id(), id);
        let params = [("Fields", "ShareLevel")];
        self.request(&path, &params).await
    }

    pub async fn get_edit_info(&self, id: &str) -> Result<Item> {
        let path = format!("Users/{}/Items/{}", self.user_id(), id);
        let params = [("Fields", "ChannelMappingInfo")];
        self.request(&path, &params).await
    }

    pub async fn get_resume(&self) -> Result<List> {
        let path = format!("Users/{}/Items/Resume", self.user_id());
        let params = [
            ("Recursive", "true"),
            (
                "Fields",
                "Overview,BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
            ("ImageTypeLimit", "1"),
            ("MediaTypes", "Video"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_image_items(&self, id: &str) -> Result<Vec<ImageItem>> {
        let path = format!("Items/{}/Images", id);
        self.request(&path, &[]).await
    }

    pub async fn image_request(
        &self,
        id: &str,
        image_type: &str,
        tag: Option<u8>,
    ) -> Result<Response> {
        let mut path = format!("Items/{}/Images/{}", id, image_type);
        if let Some(tag) = tag {
            path.push_str(&format!("/{}", tag));
        }
        let params = [
            (
                "maxHeight",
                if image_type == "Backdrop" {
                    "800"
                } else {
                    "300"
                },
            ),
            (
                "maxWidth",
                if image_type == "Backdrop" {
                    "1280"
                } else {
                    "300"
                },
            ),
        ];
        self.request_picture(&path, &params).await
    }

    pub async fn get_image(&self, id: &str, image_type: &str, tag: Option<u8>) -> Result<String> {
        match self.image_request(id, image_type, tag).await {
            Ok(response) => {
                let bytes = response.bytes().await?;

                let path = if bytes.len() > 1000 {
                    self.save_image(id, image_type, tag, &bytes)
                } else {
                    String::new()
                };

                Ok(path)
            }
            Err(e) => Err(e),
        }
    }

    pub fn save_image(&self, id: &str, image_type: &str, tag: Option<u8>, bytes: &[u8]) -> String {
        let cache_path = emby_cache_path();
        let path = format!("{}-{}-{}", id, image_type, tag.unwrap_or(0));
        let path = cache_path.join(path);
        std::fs::write(&path, bytes).unwrap();
        path.to_string_lossy().to_string()
    }

    pub async fn get_artist_albums(&self, id: &str, artist_id: &str) -> Result<List> {
        let path = format!("Users/{}/Items", self.user_id());
        let params = [
            ("IncludeItemTypes", "MusicAlbum"),
            ("Recursive", "true"),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
            ("SortBy", "ProductionYear,SortName"),
            ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
            ("SortOrder", "Descending"),
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear",
            ),
            ("AlbumArtistIds", artist_id),
            ("ExcludeItemIds", id),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_shows_next_up(&self, series_id: &str) -> Result<List> {
        let path = "Shows/NextUp".to_string();
        let params = [
            ("Fields", "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio"),
            ("Limit", "1"),
            ("ImageTypeLimit", "1"),
            ("SeriesId", series_id),
            ("UserId", &self.user_id()),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_playbackinfo(&self, id: &str) -> Result<Media> {
        let path = format!("Items/{}/PlaybackInfo", id);
        let params = [
            ("StartTimeTicks", "0"),
            ("UserId", &self.user_id()),
            ("AutoOpenLiveStream", "true"),
            ("IsPlayback", "false"),
            ("AudioStreamIndex", "1"),
            ("SubtitleStreamIndex", "1"),
            ("MaxStreamingBitrate", "160000000"),
            ("reqformat", "json"),
        ];
        let profile: Value = serde_json::from_str(PROFILE).expect("Failed to parse profile");
        self.post_json(&path, &params, profile).await
    }

    pub async fn scan(&self, id: &str) -> Result<Response> {
        let path = format!("Items/{}/Refresh", id);
        let params = [
            ("Recursive", "true"),
            ("ImageRefreshMode", "Default"),
            ("MetadataRefreshMode", "Default"),
            ("ReplaceAllImages", "false"),
            ("ReplaceAllMetadata", "false"),
        ];
        self.post(&path, &params, json!({})).await
    }

    pub async fn fullscan(
        &self,
        id: &str,
        replace_images: &str,
        replace_metadata: &str,
    ) -> Result<Response> {
        let path = format!("Items/{}/Refresh", id);
        let params = [
            ("Recursive", "true"),
            ("ImageRefreshMode", "FullRefresh"),
            ("MetadataRefreshMode", "FullRefresh"),
            ("ReplaceAllImages", replace_images),
            ("ReplaceAllMetadata", replace_metadata),
        ];
        self.post(&path, &params, json!({})).await
    }

    pub async fn remote_search(
        &self,
        type_: &str,
        info: &RemoteSearchInfo,
    ) -> Result<Vec<RemoteSearchResult>> {
        let path = format!("Items/RemoteSearch/{}", type_);
        let body = json!(info);
        self.post_json(&path, &[], body).await
    }

    pub async fn get_user_avatar(&self) -> Result<String> {
        let path = format!("Users/{}/Images/Primary", self.user_id());
        let params = [("maxHeight", "50"), ("maxWidth", "50")];
        let response = self.request_picture(&path, &params).await?;
        let bytes = response.bytes().await?;
        let path = self.save_image(&self.user_id(), "Primary", None, &bytes);
        Ok(path)
    }

    pub async fn get_external_id_info(&self, id: &str) -> Result<Vec<ExternalIdInfo>> {
        let path = format!("Items/{}/ExternalIdInfos", id);
        let params = [("IsSupportedAsIdentifier", "true")];
        self.request(&path, &params).await
    }

    pub async fn get_live_playbackinfo(&self, id: &str) -> Result<LiveMedia> {
        let path = format!("Items/{}/PlaybackInfo", id);
        let params = [
            ("StartTimeTicks", "0"),
            ("UserId", &self.user_id()),
            ("AutoOpenLiveStream", "true"),
            ("IsPlayback", "true"),
            ("MaxStreamingBitrate", "160000000"),
            ("reqformat", "json"),
        ];
        let profile: Value = serde_json::from_str(LIVEPROFILE).unwrap();
        self.post_json(&path, &params, profile).await
    }

    pub async fn get_sub(&self, id: &str, source_id: &str) -> Result<Media> {
        let path = format!("Items/{}/PlaybackInfo", id);
        let params = [
            ("StartTimeTicks", "0"),
            ("UserId", &self.user_id()),
            ("AutoOpenLiveStream", "true"),
            ("IsPlayback", "true"),
            ("AudioStreamIndex", "1"),
            ("SubtitleStreamIndex", "1"),
            ("MediaSourceId", source_id),
            ("MaxStreamingBitrate", "4000000"),
            ("reqformat", "json"),
        ];
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        self.post_json(&path, &params, profile).await
    }

    pub async fn get_library(&self) -> Result<List> {
        let path = format!("Users/{}/Views", &self.user_id());
        self.request(&path, &[]).await
    }

    pub async fn get_latest(&self, id: &str) -> Result<Vec<SimpleListItem>> {
        let path = format!("Users/{}/Items/Latest", &self.user_id());
        let params = [
            ("Limit", "16"),
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("ParentId", id),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
        ];
        self.request(&path, &params).await
    }

    pub fn get_streaming_url(&self, path: &str) -> String {
        let url = self.url.lock().unwrap().as_ref().unwrap().clone();
        url.join(path.trim_start_matches('/')).unwrap().to_string()
    }

    pub async fn get_list(
        &self,
        id: &str,
        start: u32,
        include_item_types: &str,
        list_type: ListType,
        sort_order: &str,
        sortby: &str,
    ) -> Result<List> {
        let user_id = &self.user_id();
        let path = match list_type {
            ListType::All => format!("Users/{}/Items", user_id),
            ListType::Resume => format!("Users/{}/Items/Resume", user_id),
            ListType::Genres => "Genres".to_string(),
            _ => format!("Users/{}/Items", user_id),
        };
        let include_item_type = match list_type {
            ListType::Tags => "Tag",
            ListType::BoxSet => "BoxSet",
            _ => include_item_types,
        };
        let start_string = start.to_string();
        let params = match list_type {
            ListType::All | ListType::Liked | ListType::Tags | ListType::BoxSet => {
                vec![
                    ("Limit", "50"),
                    (
                        "Fields",
                        "Overview,BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
                    ),
                    ("ParentId", id),
                    ("ImageTypeLimit", "1"),
                    ("StartIndex", &start_string),
                    ("Recursive", "true"),
                    ("IncludeItemTypes", include_item_type),
                    ("SortBy", sortby),
                    ("SortOrder", sort_order),
                    ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
                    if list_type == ListType::Liked {("Filters", "IsFavorite")} else {("", "")},
                ]
            }
            ListType::Resume => {
                vec![
                    (
                        "Fields",
                        "Overview,BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear",
                    ),
                    ("ParentId", id),
                    ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
                    ("ImageTypeLimit", "1"),
                    (
                        "IncludeItemTypes",
                        match include_item_type {
                            "Series" => "Episode",
                            _ => include_item_type,
                        },
                    ),
                    ("Limit", "30"),
                ]
            }
            ListType::Genres => vec![
                ("Fields", "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio"),
                ("IncludeItemTypes", include_item_type),
                ("StartIndex", &start_string),
                ("ImageTypeLimit", "1"),
                ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
                ("Limit", "50"),
                ("userId", user_id),
                ("Recursive", "true"),
                ("ParentId", id),
            ],
            _ => vec![],
        };
        self.request(&path, &params).await
    }

    pub async fn get_inlist(
        &self,
        id: Option<String>,
        start: u32,
        listtype: &str,
        parentid: &str,
        sort_order: &str,
        sortby: &str,
    ) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id());
        let start_string = start.to_string();
        let mut params = vec![
            ("Limit", "50"),
            (
                "Fields",
                "Overview,BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("ImageTypeLimit", "1"),
            ("StartIndex", &start_string),
            ("Recursive", "true"),
            ("IncludeItemTypes", "Movie,Series,MusicAlbum"),
            ("SortBy", sortby),
            ("SortOrder", sort_order),
            ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
            if listtype == "Genre" {
                ("GenreIds", parentid)
            } else if listtype == "Studios" {
                ("StudioIds", parentid)
            } else {
                ("TagIds", parentid)
            },
        ];
        let id_clone;
        if let Some(id) = id {
            id_clone = id.clone();
            params.push(("ParentId", &id_clone));
        }
        self.request(&path, &params).await
    }

    pub async fn like(&self, id: &str) -> Result<()> {
        let path = format!(
            "Users/{}/FavoriteItems/{}",
            &self.user_id.lock().unwrap(),
            id
        );
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn unlike(&self, id: &str) -> Result<()> {
        let path = format!(
            "Users/{}/FavoriteItems/{}/Delete",
            &self.user_id.lock().unwrap(),
            id
        );
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn set_as_played(&self, id: &str) -> Result<()> {
        let path = format!("Users/{}/PlayedItems/{}", &self.user_id(), id);
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn set_as_unplayed(&self, id: &str) -> Result<()> {
        let path = format!(
            "Users/{}/PlayedItems/{}/Delete",
            &self.user_id.lock().unwrap(),
            id
        );
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn position_back(&self, back: &Back, backtype: BackType) -> Result<()> {
        let path = match backtype {
            BackType::Start => "Sessions/Playing".to_string(),
            BackType::Stop => "Sessions/Playing/Stopped".to_string(),
            BackType::Back => "Sessions/Playing/Progress".to_string(),
        };
        let params = [("reqformat", "json")];
        let body = json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate": 400000000u64,"PositionTicks":back.tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
        self.post(&path, &params, body).await?;
        Ok(())
    }

    pub async fn get_similar(&self, id: &str) -> Result<List> {
        let path = format!("Items/{}/Similar", id);
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("UserId", &self.user_id()),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_person(&self, id: &str, types: &str) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id());
        let params = [
            (
                "Fields",
                "PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("PersonIds", id),
            ("Recursive", "true"),
            ("CollapseBoxSetItems", "false"),
            ("SortBy", "SortName"),
            ("SortOrder", "Ascending"),
            ("IncludeItemTypes", types),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_person_large_list(
        &self,
        id: &str,
        types: &str,
        sort_by: &str,
        sort_order: &str,
        start_index: u32,
    ) -> Result<List> {
        let start_string = start_index.to_string();
        let path = format!("Users/{}/Items", &self.user_id());
        let params = [
            (
                "Fields",
                "Overview,PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("PersonIds", id),
            ("Recursive", "true"),
            ("CollapseBoxSetItems", "false"),
            ("SortBy", sort_by),
            ("SortOrder", sort_order),
            ("IncludeItemTypes", types),
            ("StartIndex", &start_string),
            ("ImageTypeLimit", "1"),
            ("Limit", "50"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_continue_play_list(&self, parent_id: &str) -> Result<List> {
        let path = "Shows/NextUp".to_string();
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,Overview",
            ),
            ("Limit", "40"),
            ("ImageTypeLimit", "1"),
            ("SeriesId", parent_id),
            ("UserId", &self.user_id()),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_season_list(&self, parent_id: &str) -> Result<List> {
        let path = format!("Shows/{}/Seasons", parent_id);
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,Overview",
            ),
            ("UserId", &self.user_id()),
            ("EnableUserData", "false"),
            ("EnableTotalRecordCount", "false"),
            ("EnableImages", "false"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_search_recommend(&self) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id());
        let params = [
            ("Limit", "20"),
            ("EnableTotalRecordCount", "false"),
            ("ImageTypeLimit", "0"),
            ("Recursive", "true"),
            ("IncludeItemTypes", "Movie,Series"),
            ("SortBy", "IsFavoriteOrLiked,Random"),
            ("Recursive", "true"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_favourite(
        &self,
        types: &str,
        start: u32,
        limit: u32,
        sort_by: &str,
        sort_order: &str,
    ) -> Result<List> {
        let user_id = {
            let user_id = self.user_id.lock().unwrap();
            user_id.to_owned()
        };
        let path = if types == "People" {
            "Persons".to_string()
        } else {
            format!("Users/{}/Items", user_id)
        };
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("Filters", "IsFavorite"),
            ("Recursive", "true"),
            ("CollapseBoxSetItems", "false"),
            ("SortBy", sort_by),
            ("SortOrder", sort_order),
            ("IncludeItemTypes", types),
            ("Limit", &limit.to_string()),
            ("StartIndex", &start.to_string()),
            if types == "People" {
                ("UserId", &user_id)
            } else {
                ("", "")
            },
        ];
        self.request(&path, &params).await
    }

    pub async fn get_included(&self, id: &str) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id());
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,CommunityRating",
            ),
            ("Limit", "12"),
            ("ListItemIds", id),
            ("Recursive", "true"),
            ("IncludeItemTypes", "Playlist,BoxSet"),
            ("SortBy", "SortName"),
            ("Recursive", "true"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_includedby(&self, parent_id: &str) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id());
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("ImageTypeLimit", "1"),
            ("ParentId", parent_id),
            ("SortBy", "DisplayOrder"),
            ("SortOrder", "Ascending"),
            ("EnableTotalRecordCount", "false"),
            ("X-Emby-Client", "Tsukimi"),
        ];
        self.request(&path, &params).await
    }

    pub async fn change_password(&self, new_password: &str) -> Result<()> {
        let path = format!("Users/{}/Password", &self.user_id());

        let old_password = match self.user_password.lock() {
            Ok(guard) => guard.to_string(),
            Err(_) => return Err(anyhow::anyhow!("Failed to acquire lock on user password")),
        };

        let body = json!({
            "CurrentPw": old_password,
            "NewPw": new_password
        });

        self.post(&path, &[], body).await?;
        Ok(())
    }

    pub async fn hide_from_resume(&self, id: &str) -> Result<()> {
        let path = format!("Users/{}/Items/{}/HideFromResume", &self.user_id(), id);
        let params = [("Hide", "true")];
        self.post(&path, &params, json!({})).await?;
        Ok(())
    }

    pub async fn get_songs(&self, parent_id: &str) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id());
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,SyncStatus",
            ),
            ("ImageTypeLimit", "1"),
            ("ParentId", parent_id),
            ("EnableTotalRecordCount", "false"),
        ];
        self.request(&path, &params).await
    }

    pub fn get_song_streaming_uri(&self, id: &str) -> String {
        let url = self.url.lock().unwrap().as_ref().unwrap().clone();

        url.join(&format!("Audio/{}/universal?UserId={}&DeviceId={}&MaxStreamingBitrate=4000000&Container=opus,mp3|mp3,mp2,mp3|mp2,m4a|aac,mp4|aac,flac,webma,webm,wav|PCM_S16LE,wav|PCM_S24LE,ogg&TranscodingContainer=aac&TranscodingProtocol=hls&AudioCodec=aac&api_key={}&PlaySessionId=1715006733496&StartTimeTicks=0&EnableRedirection=true&EnableRemoteMedia=false",
        id, &self.user_id(), &DEVICE_ID.to_string(), self.user_access_token.lock().unwrap(), )).unwrap().to_string()
    }

    fn user_id(&self) -> String {
        self.user_id.lock().unwrap().to_string()
    }

    pub async fn get_additional(&self, id: &str) -> Result<List> {
        let path = format!("Videos/{}/AdditionalParts", id);
        let params: [(&str, &str); 1] = [("UserId", &self.user_id())];
        self.request(&path, &params).await
    }

    pub async fn get_channels(&self) -> Result<List> {
        let params = [
            ("IsAiring", "true"),
            ("userId", &self.user_id()),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
            ("Fields", "ProgramPrimaryImageAspectRatio"),
            ("SortBy", "DefaultChannelOrder"),
            ("SortOrder", "Ascending"),
        ];
        self.request("LiveTv/Channels", &params).await
    }

    pub async fn get_channels_list(&self, start_index: u32) -> Result<List> {
        let params = [
            ("IsAiring", "true"),
            ("userId", &self.user_id()),
            ("ImageTypeLimit", "1"),
            ("Limit", "50"),
            ("Fields", "ProgramPrimaryImageAspectRatio"),
            ("SortBy", "DefaultChannelOrder"),
            ("SortOrder", "Ascending"),
            ("StartIndex", &start_index.to_string()),
        ];
        self.request("LiveTv/Channels", &params).await
    }

    pub async fn get_server_info(&self) -> Result<ServerInfo> {
        self.request("System/Info", &[]).await
    }

    pub async fn get_server_info_public(&self) -> Result<PublicServerInfo> {
        self.request("System/Info/Public", &[]).await
    }

    pub async fn shut_down(&self) -> Result<Response> {
        self.post("System/Shutdown", &[], json!({})).await
    }

    pub async fn restart(&self) -> Result<Response> {
        self.post("System/Restart", &[], json!({})).await
    }

    pub async fn get_activity_log(&self, has_user_id: bool) -> Result<ActivityLogs> {
        let params = [
            ("Limit", "15"),
            ("StartIndex", "0"),
            ("hasUserId", &has_user_id.to_string()),
        ];
        self.request("System/ActivityLog/Entries", &params).await
    }

    pub async fn get_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>> {
        self.request("ScheduledTasks", &[]).await
    }

    pub async fn run_scheduled_task(&self, id: String) -> Result<()> {
        let path = format!("ScheduledTasks/Running/{}", &id);
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub fn get_image_path(&self, id: &str, image_type: &str, image_index: Option<u32>) -> String {
        let path = format!("Items/{}/Images/{}/", id, image_type);
        let url = self
            .url
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .clone()
            .join(&path)
            .unwrap();
        match image_index {
            Some(index) => url.join(&index.to_string()).unwrap().to_string(),
            None => url.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::error::UserFacingError;

    use super::*;

    #[tokio::test]
    async fn search() {
        let _ = EMBY_CLIENT.header_change_url("https://example.com", "443");
        let result = EMBY_CLIENT.login("test", "test").await;
        match result {
            Ok(response) => {
                println!("{}", response.access_token);
                let _ = EMBY_CLIENT.header_change_token(&response.access_token);
                let _ = EMBY_CLIENT.set_user_id(&response.user.id);
            }
            Err(e) => {
                eprintln!("{}", e.to_user_facing());
            }
        }

        let result = EMBY_CLIENT.search("你的名字", &["Movie"], "0");
        match result.await {
            Ok(items) => {
                for item in items.items {
                    println!("{}", item.name);
                }
            }
            Err(e) => {
                eprintln!("{}", e.to_user_facing());
            }
        }
    }

    #[test]
    fn parse_url() {
        let uri = "127.0.0.1";
        let url = if Url::parse(uri).is_err() {
            format!("http://{}", uri)
        } else {
            uri.to_string()
        };

        assert_eq!(url, "http://127.0.0.1");
    }
}
