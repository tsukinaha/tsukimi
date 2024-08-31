use std::{env, sync::Mutex};

use reqwest::{header::HeaderValue, Method, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, warn};
use url::Url;
use uuid::Uuid;

use crate::{
    config::{proxy::ReqClient, Account, APP_VERSION},
    ui::models::emby_cache_path,
    utils::{spawn, spawn_tokio},
};

use once_cell::sync::Lazy;

use super::structs::{
    ActivityLogs, AuthenticateResponse, Back, ExternalIdInfo, ImageItem, Item, List, LiveMedia,
    LoginResponse, Media, RemoteSearchInfo, RemoteSearchResult, ScheduledTask, SerInList,
    ServerInfo, SimpleListItem,
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
        }
    }

    pub fn init(&self, account: &Account) {
        self.header_change_url(&account.server, &account.port);
        self.header_change_token(&account.access_token);
        self.set_user_id(&account.user_id);
        env::set_var("EMBY_NAME", &account.servername);
        env::set_var("EMBY_USERNAME", &account.username);
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
    }

    pub fn header_change_token(&self, token: &str) {
        let mut headers = self.headers.lock().unwrap();
        headers.insert("X-Emby-Token", HeaderValue::from_str(token).unwrap());
    }

    pub fn header_change_url(&self, url: &str, port: &str) {
        let mut url = Url::parse(url).unwrap();
        url.set_port(Some(port.parse::<u16>().unwrap_or_default()))
            .unwrap();
        let mut url_lock = self.url.lock().unwrap();
        *url_lock = Some(url.join("emby/").unwrap());
    }

    pub fn set_user_id(&self, user_id: &str) {
        let mut user_id_lock = self.user_id.lock().unwrap();
        *user_id_lock = user_id.to_string();
    }

    pub fn get_url_and_headers(&self) -> (Url, reqwest::header::HeaderMap) {
        let url = self.url.lock().unwrap().as_ref().unwrap().clone();
        let headers = self.headers.lock().unwrap().clone();
        (url, headers)
    }

    pub async fn request<T>(&self, path: &str, params: &[(&str, &str)]) -> Result<T, reqwest::Error>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let request = self.prepare_request(Method::GET, path, params);
        let res = self.send_request(request).await?;

        match res.error_for_status() {
            Ok(res) => Ok(res.json().await?),
            Err(e) => Err(e),
        }
    }

    pub async fn request_picture(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<Response, reqwest::Error> {
        let request = self.prepare_request(Method::GET, path, params);
        request.send().await
    }

    pub async fn post<B>(
        &self,
        path: &str,
        params: &[(&str, &str)],
        body: B,
    ) -> Result<Response, reqwest::Error>
    where
        B: Serialize,
    {
        let request = self.prepare_request(Method::POST, path, params).json(&body);
        self.send_request(request).await
    }

    fn prepare_request(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, &str)],
    ) -> RequestBuilder {
        let (mut url, headers) = self.get_url_and_headers();
        url = url.join(path).unwrap();
        self.add_params_to_url(&mut url, params);
        self.client.request(method, url).headers(headers)
    }

    async fn send_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<Response, reqwest::Error> {
        let res = request.send().await?;
        Ok(res)
    }

    pub async fn authenticate_admin(&self) -> Result<AuthenticateResponse, reqwest::Error> {
        let path = format!("Users/{}", self.user_id());
        self.request(&path, &[]).await
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<LoginResponse, reqwest::Error> {
        let body = json!({
            "Username": username,
            "Pw": password
        });
        self.post("Users/authenticatebyname", &[], body)
            .await?
            .json()
            .await
    }

    pub fn add_params_to_url(&self, url: &mut Url, params: &[(&str, &str)]) {
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, value);
        }
        info!("Request URL: {}", url);
    }

    pub async fn search(
        &self,
        query: &str,
        filter: &[&str],
        start_index: &str,
    ) -> Result<List, reqwest::Error> {
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
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
            ("ImageTypeLimit", "1"),
            ("Recursive", "true"),
            ("SearchTerm", query),
            ("GroupProgramsBySeries", "true"),
            ("Limit", "50"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_series_info(&self, id: &str) -> Result<SerInList, reqwest::Error> {
        let path = format!("Shows/{}/Episodes", id);
        let params = [
            ("Fields", "Overview"),
            ("EnableTotalRecordCount", "true"),
            ("EnableImages", "true"),
            ("UserId", &self.user_id()),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_item_info(&self, id: &str) -> Result<Item, reqwest::Error> {
        let path = format!("Users/{}/Items/{}", self.user_id(), id);
        let params = [("Fields", "ShareLevel")];
        self.request(&path, &params).await
    }

    pub async fn get_edit_info(&self, id: &str) -> Result<Item, reqwest::Error> {
        let path = format!("Users/{}/Items/{}", self.user_id(), id);
        let params = [("Fields", "ChannelMappingInfo")];
        self.request(&path, &params).await
    }

    pub async fn get_resume(&self) -> Result<List, reqwest::Error> {
        let path = format!("Users/{}/Items/Resume", self.user_id());
        let params = [
            ("Recursive", "true"),
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
            ("ImageTypeLimit", "1"),
            ("MediaTypes", "Video"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_image_items(&self, id: &str) -> Result<Vec<ImageItem>, reqwest::Error> {
        let path = format!("Items/{}/Images", id);
        self.request(&path, &[]).await
    }

    pub async fn image_request(
        &self,
        id: &str,
        image_type: &str,
        tag: Option<u8>,
    ) -> Result<Response, reqwest::Error> {
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
                    "600"
                },
            ),
        ];
        self.request_picture(&path, &params).await
    }

    pub async fn get_image(
        &self,
        id: &str,
        image_type: &str,
        tag: Option<u8>,
    ) -> Result<String, reqwest::Error> {
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

    pub async fn get_artist_albums(
        &self,
        id: &str,
        artist_id: &str,
    ) -> Result<List, reqwest::Error> {
        let path = format!("Users/{}/Items", self.user_id());
        let params = [
            ("IncludeItemTypes", "MusicAlbum"),
            ("Recursive", "true"),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
            ("SortBy", "ProductionYear,SortName"),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
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

    pub async fn get_playbackinfo(&self, id: &str) -> Result<Media, reqwest::Error> {
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
        let profile: Value = serde_json::from_str(PROFILE).unwrap();
        self.post(&path, &params, profile).await?.json().await
    }

    pub async fn scan(&self, id: &str) -> Result<Response, reqwest::Error> {
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
    ) -> Result<Response, reqwest::Error> {
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
    ) -> Result<Vec<RemoteSearchResult>, reqwest::Error> {
        let path = format!("Items/RemoteSearch/{}", type_);
        println!("{}", path);
        let body = json!(info);
        self.post(&path, &[], body).await?.json().await
    }

    pub async fn get_external_id_info(
        &self,
        id: &str,
    ) -> Result<Vec<ExternalIdInfo>, reqwest::Error> {
        let path = format!("Items/{}/ExternalIdInfos", id);
        let params = [("IsSupportedAsIdentifier", "true")];
        self.request(&path, &params).await
    }

    pub async fn get_live_playbackinfo(&self, id: &str) -> Result<LiveMedia, reqwest::Error> {
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
        self.post(&path, &params, profile).await?.json().await
    }

    pub async fn get_sub(&self, id: &str, source_id: &str) -> Result<Media, reqwest::Error> {
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
        self.post(&path, &params, profile).await?.json().await
    }

    pub async fn get_library(&self) -> Result<List, reqwest::Error> {
        let path = format!("Users/{}/Views", &self.user_id());
        self.request(&path, &[]).await
    }

    pub async fn get_latest(&self, id: &str) -> Result<Vec<SimpleListItem>, reqwest::Error> {
        let path = format!("Users/{}/Items/Latest", &self.user_id());
        let params = [
            ("Limit", "16"),
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("ParentId", id),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ];
        self.request(&path, &params).await
    }

    pub fn get_streaming_url(&self, path: &str) -> String {
        let url = self.url.lock().unwrap().as_ref().unwrap().clone();
        url.join(path.trim_start_matches('/')).unwrap().to_string()
    }

    pub async fn get_list(
        &self,
        id: String,
        start: &str,
        include_item_types: &str,
        listtype: &str,
        sort_order: &str,
        sortby: &str,
    ) -> Result<List, reqwest::Error> {
        let user_id = &self.user_id();
        let path = match listtype {
            "item" => format!("Users/{}/Items", user_id),
            "resume" => format!("Users/{}/Items/Resume", user_id),
            "genres" => "Genres".to_string(),
            _ => format!("Users/{}/Items", user_id),
        };
        let include_item_type = match listtype {
            "tags" => "Tag",
            "boxset" => "BoxSet",
            _ => include_item_types,
        };
        let params = match listtype {
            "all" | "liked" | "tags" | "boxset" => {
                vec![
                    ("Limit", "50"),
                    (
                        "Fields",
                        "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
                    ),
                    ("ParentId", &id),
                    ("ImageTypeLimit", "1"),
                    ("StartIndex", start),
                    ("Recursive", "true"),
                    ("IncludeItemTypes", include_item_type),
                    ("SortBy", sortby),
                    ("SortOrder", sort_order),
                    ("EnableImageTypes", "Primary,Backdrop,Thumb"),
                    if listtype == "liked" {("Filters", "IsFavorite")} else {("", "")},
                ]
            }
            "resume" => {
                vec![
                    (
                        "Fields",
                        "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear",
                    ),
                    ("ParentId", &id),
                    ("EnableImageTypes", "Primary,Backdrop,Thumb"),
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
            "genres" => vec![
                ("Fields", "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio"),
                ("IncludeItemTypes", include_item_type),
                ("StartIndex", start),
                ("ImageTypeLimit", "1"),
                ("EnableImageTypes", "Primary,Backdrop,Thumb"),
                ("Limit", "50"),
                ("userId", user_id),
                ("Recursive", "true"),
                ("ParentId", &id),
                ("SortBy", sortby),
                ("SortOrder", sort_order),
            ],
            _ => vec![],
        };
        self.request(&path, &params).await
    }

    pub async fn get_inlist(
        &self,
        id: Option<String>,
        start: &str,
        listtype: &str,
        parentid: &str,
        sort_order: &str,
        sortby: &str,
    ) -> Result<List, reqwest::Error> {
        let path = format!("Users/{}/Items", &self.user_id());
        let mut params = vec![
            ("Limit", "50"),
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("ImageTypeLimit", "1"),
            ("StartIndex", start),
            ("Recursive", "true"),
            ("IncludeItemTypes", "Movie,Series,MusicAlbum"),
            ("SortBy", sortby),
            ("SortOrder", sort_order),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
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

    pub async fn like(&self, id: &str) -> Result<(), reqwest::Error> {
        let path = format!(
            "Users/{}/FavoriteItems/{}",
            &self.user_id.lock().unwrap(),
            id
        );
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn unlike(&self, id: &str) -> Result<(), reqwest::Error> {
        let path = format!(
            "Users/{}/FavoriteItems/{}/Delete",
            &self.user_id.lock().unwrap(),
            id
        );
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn set_as_played(&self, id: &str) -> Result<(), reqwest::Error> {
        let path = format!("Users/{}/PlayedItems/{}", &self.user_id(), id);
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn set_as_unplayed(&self, id: &str) -> Result<(), reqwest::Error> {
        let path = format!(
            "Users/{}/PlayedItems/{}/Delete",
            &self.user_id.lock().unwrap(),
            id
        );
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn position_back(
        &self,
        back: &Back,
        backtype: BackType,
    ) -> Result<(), reqwest::Error> {
        let path = match backtype {
            BackType::Start => "Sessions/Playing".to_string(),
            BackType::Stop => "Sessions/Playing/Stopped".to_string(),
            BackType::Back => "Sessions/Playing/Progress".to_string(),
        };
        let params = [("reqformat", "json")];
        let body = json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":back.tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
        self.post(&path, &params, body).await?;
        Ok(())
    }

    pub async fn get_similar(&self, id: &str) -> Result<List, reqwest::Error> {
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

    pub async fn get_person(&self, id: &str, types: &str) -> Result<List, reqwest::Error> {
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

    pub async fn get_search_recommend(&self) -> Result<List, reqwest::Error> {
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
    ) -> Result<List, reqwest::Error> {
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
            ("SortBy", "SortName"),
            ("SortOrder", "Ascending"),
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

    pub async fn get_included(&self, id: &str) -> Result<List, reqwest::Error> {
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

    pub async fn get_includedby(&self, parent_id: &str) -> Result<List, reqwest::Error> {
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

    pub async fn change_password(&self, new_password: &str) -> Result<(), reqwest::Error> {
        let path = format!("Users/{}/Password", &self.user_id());
        let old_password = std::env::var("EMBY_PASSWORD").unwrap_or("".to_string());
        let body = json!({
            "CurrentPw": old_password,
            "NewPw": new_password
        });
        self.post(&path, &[], body).await?;
        Ok(())
    }

    pub async fn hide_from_resume(&self, id: &str) -> Result<(), reqwest::Error> {
        let path = format!(
            "Users/{}/Items/{}/HideFromResume",
            &self.user_id.lock().unwrap(),
            id
        );
        let params = [("Hide", "true")];
        self.post(&path, &params, json!({})).await?;
        Ok(())
    }

    pub async fn get_songs(&self, parent_id: &str) -> Result<List, reqwest::Error> {
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
        id, &self.user_id(), &DEVICE_ID.to_string(), std::env::var("EMBY_ACCESS_TOKEN").unwrap(), )).unwrap().to_string()
    }

    pub async fn get_random(&self) -> Result<List, reqwest::Error> {
        let path = format!("Users/{}/Items", &self.user_id());
        let params = [
            ("Fields", "ProductionYear,CommunityRating"),
            ("EnableImageTypes", "Logo,Backdrop"),
            ("ImageTypeLimit", "1"),
            ("EnableTotalRecordCount", "false"),
            ("SortBy", "Random"),
            ("Limit", "10"),
            ("Recursive", "true"),
            ("IncludeItemTypes", "Series"),
            ("EnableUserData", "false"),
        ];
        self.request(&path, &params).await
    }

    fn user_id(&self) -> String {
        self.user_id.lock().unwrap().to_string()
    }

    pub async fn get_additional(&self, id: &str) -> Result<List, reqwest::Error> {
        let path = format!("Videos/{}/AdditionalParts", id);
        let params: [(&str, &str); 1] = [("UserId", &self.user_id())];
        self.request(&path, &params).await
    }

    pub async fn get_channels(&self) -> Result<List, reqwest::Error> {
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

    pub async fn get_channels_list(&self, start_index: &str) -> Result<List, reqwest::Error> {
        let params = [
            ("IsAiring", "true"),
            ("userId", &self.user_id()),
            ("ImageTypeLimit", "1"),
            ("Limit", "50"),
            ("Fields", "ProgramPrimaryImageAspectRatio"),
            ("SortBy", "DefaultChannelOrder"),
            ("SortOrder", "Ascending"),
            ("StartIndex", start_index),
        ];
        self.request("LiveTv/Channels", &params).await
    }

    pub async fn get_server_info(&self) -> Result<ServerInfo, reqwest::Error> {
        self.request("System/Info", &[]).await
    }

    pub async fn shut_down(&self) -> Result<Response, reqwest::Error> {
        self.post("System/Shutdown", &[], json!({})).await
    }

    pub async fn restart(&self) -> Result<Response, reqwest::Error> {
        self.post("System/Restart", &[], json!({})).await
    }

    pub async fn get_activity_log(
        &self,
        has_user_id: bool,
    ) -> Result<ActivityLogs, reqwest::Error> {
        let params = [
            ("Limit", "15"),
            ("StartIndex", "0"),
            ("hasUserId", &has_user_id.to_string()),
        ];
        self.request("System/ActivityLog/Entries", &params).await
    }

    pub async fn get_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>, reqwest::Error> {
        self.request("ScheduledTasks", &[]).await
    }

    pub async fn run_scheduled_task(&self, id: String) -> Result<(), reqwest::Error> {
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
        EMBY_CLIENT.header_change_url("https://example.com", "443");
        let result = EMBY_CLIENT.login("test", "test").await;
        match result {
            Ok(response) => {
                println!("{}", response.access_token);
                EMBY_CLIENT.header_change_token(&response.access_token);
                EMBY_CLIENT.set_user_id(&response.user.id);
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
}
