use std::{
    hash::Hasher,
    sync::Arc,
};

use anyhow::{
    Result,
    anyhow,
};
use once_cell::sync::Lazy;
use reqwest::{
    Client,
    Method,
    RequestBuilder,
    Response,
    header::HeaderValue,
};
use serde::{
    Deserialize,
    Serialize,
    de::DeserializeOwned,
};
use serde_json::{
    Value,
    json,
};
use tokio::sync::Mutex;
use tracing::{
    debug,
    warn,
};
use url::Url;
use uuid::Uuid;

#[cfg(target_os = "windows")]
use super::windows_compat::xattr;
use super::{
    Account,
    ReqClient,
    error::UserFacingError,
    structs::{
        ActivityLogs,
        AuthenticateResponse,
        Back,
        DeleteInfo,
        ExternalIdInfo,
        FilterList,
        ImageItem,
        ImageSearchResult,
        List,
        LoginResponse,
        Media,
        MissingEpisodesList,
        PublicServerInfo,
        RemoteSearchInfo,
        ScheduledTask,
        ServerInfo,
        SimpleListItem,
    },
};
use crate::{
    CLIENT_ID,
    config::VERSION,
    ui::{
        SETTINGS,
        jellyfin_cache_path,
        widgets::{
            filter_panel::FiltersList,
            single_grid::imp::ListType,
        },
    },
    utils::spawn_tokio_without_await,
};

pub static JELLYFIN_CLIENT: Lazy<JellyfinClient> = Lazy::new(JellyfinClient::default);
pub static DEVICE_ID: Lazy<String> = Lazy::new(|| {
    let uuid = SETTINGS.device_uuid();
    if uuid.is_empty() {
        let uuid = Uuid::new_v4().to_string();
        let _ = SETTINGS.set_device_uuid(&uuid);
        uuid
    } else {
        uuid
    }
});

const PROFILE: &str = include_str!("stream_profile.json");

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

pub struct JellyfinClient {
    pub url: Mutex<Option<Url>>,
    pub client: Client,
    pub semaphore: Arc<tokio::sync::Semaphore>,
    pub headers: Mutex<reqwest::header::HeaderMap>,
    pub user_id: Mutex<String>,
    pub user_name: Mutex<String>,
    pub user_password: Mutex<String>,
    pub user_access_token: Mutex<String>,
    pub server_name: Mutex<String>,
    pub server_name_hash: Mutex<String>,
}

fn generate_jellyfin_authorization(
    user_id: &str, client: &str, device: &str, device_id: &str, version: &str,
) -> String {
    format!(
        "Emby UserId={user_id},Client={client},Device={device},DeviceId={device_id},Version={version}"
    )
}

fn generate_hash(s: &str) -> String {
    let mut hasher = fnv::FnvHasher::default();
    hasher.write(s.as_bytes());
    format!("{:x}", hasher.finish())
}

impl Default for JellyfinClient {
    fn default() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));
        headers.insert(
            "X-Emby-authorization",
            HeaderValue::from_str(&generate_jellyfin_authorization(
                "",
                CLIENT_ID,
                &DEVICE_NAME,
                &DEVICE_ID,
                VERSION,
            ))
            .unwrap(),
        );
        Self {
            url: Mutex::new(None),
            client: ReqClient::build(),
            semaphore: Arc::new(tokio::sync::Semaphore::new(SETTINGS.threads() as usize)),
            headers: Mutex::new(headers),
            user_id: Mutex::new(String::new()),
            user_name: Mutex::new(String::new()),
            user_password: Mutex::new(String::new()),
            user_access_token: Mutex::new(String::new()),
            server_name: Mutex::new(String::new()),
            server_name_hash: Mutex::new(String::new()),
        }
    }
}

impl JellyfinClient {
    pub async fn init(&self, account: &Account) -> Result<(), Box<dyn std::error::Error>> {
        self.header_change_url(&account.server, &account.port)
            .await?;
        self.header_change_token(&account.access_token).await?;
        self.set_user_id(&account.user_id).await?;
        self.set_user_name(&account.username).await?;
        self.set_user_password(&account.password).await?;
        self.set_user_access_token(&account.access_token).await?;
        self.set_server_name(&account.servername).await?;
        crate::ui::provider::set_admin(false);
        spawn_tokio_without_await(async move {
            match JELLYFIN_CLIENT.authenticate_admin().await {
                Ok(r) => {
                    if r.policy.is_administrator {
                        crate::ui::provider::set_admin(true);
                    }
                }
                Err(e) => warn!("Failed to authenticate as admin: {}", e),
            }
        });
        Ok(())
    }

    pub async fn header_change_token(&self, token: &str) -> Result<()> {
        let mut headers = self.headers.lock().await;
        headers.insert("X-Emby-Token", HeaderValue::from_str(token)?);
        Ok(())
    }

    pub async fn header_change_url(&self, url: &str, port: &str) -> Result<()> {
        let mut url = Url::parse(url)?;
        url.set_port(Some(port.parse::<u16>().unwrap_or_default()))
            .map_err(|_| anyhow!("Failed to set port"))?;
        let mut url_lock = self.url.lock().await;
        *url_lock = Some(url.join("emby/")?);
        Ok(())
    }

    pub async fn set_user_id(&self, user_id: &str) -> Result<()> {
        let mut user_id_lock = self.user_id.lock().await;
        *user_id_lock = user_id.to_string();
        self.header_change_user_id(user_id).await?;
        Ok(())
    }

    pub async fn header_change_user_id(&self, user_id: &str) -> Result<()> {
        let mut headers = self.headers.lock().await;
        headers.insert(
            "X-Emby-authorization",
            HeaderValue::from_str(&generate_jellyfin_authorization(
                user_id,
                CLIENT_ID,
                &DEVICE_NAME,
                &DEVICE_ID,
                VERSION,
            ))?,
        );
        Ok(())
    }

    pub async fn set_user_name(&self, user_name: &str) -> Result<()> {
        let mut user_name_lock = self.user_name.lock().await;
        *user_name_lock = user_name.to_string();
        Ok(())
    }

    pub async fn set_user_password(&self, user_password: &str) -> Result<()> {
        let mut user_password_lock = self.user_password.lock().await;
        *user_password_lock = user_password.to_string();
        Ok(())
    }

    pub async fn set_user_access_token(&self, user_access_token: &str) -> Result<()> {
        let mut user_access_token_lock = self.user_access_token.lock().await;
        *user_access_token_lock = user_access_token.to_string();
        Ok(())
    }

    pub async fn set_server_name(&self, server_name: &str) -> Result<()> {
        let mut server_name_lock = self.server_name.lock().await;
        *server_name_lock = server_name.to_string();

        let mut server_name_hash_lock = self.server_name_hash.lock().await;

        *server_name_hash_lock = generate_hash(server_name);
        Ok(())
    }

    pub async fn get_url_and_headers(&self) -> Result<(Url, reqwest::header::HeaderMap)> {
        let url = self
            .url
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| anyhow!("URL is not set"))?
            .to_owned();
        let headers = self.headers.lock().await.to_owned();
        Ok((url, headers))
    }

    pub async fn request<T>(&self, path: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let request = self.prepare_request(Method::GET, path, params).await?;
        let res = self.send_request(request).await?;

        let res = match res.error_for_status() {
            Ok(r) => r,
            Err(e) => {
                let Some(status) = e.status() else {
                    return Err(anyhow!("Failed to get status"));
                };
                return Err(anyhow!("{}", status));
            }
        };

        let res_text = res.text().await?;
        match serde_json::from_str(&res_text) {
            Ok(json) => Ok(json),
            Err(e) => Err(anyhow!(
                "Request Path: {}\nFailed parsing response to json {}: {}",
                path,
                e,
                res_text
            )),
        }
    }

    pub async fn request_picture(
        &self, path: &str, params: &[(&str, &str)], etag: Option<String>,
    ) -> Result<Response> {
        let request = self
            .prepare_request(Method::GET, path, params)
            .await?
            .header("If-None-Match", etag.unwrap_or_default());
        let res = request.send().await?;
        Ok(res)
    }

    pub async fn post<B>(&self, path: &str, params: &[(&str, &str)], body: B) -> Result<Response>
    where
        B: Serialize,
    {
        let request = self
            .prepare_request(Method::POST, path, params)
            .await?
            .json(&body);
        let res = self.send_request(request).await?;
        Ok(res)
    }

    pub async fn post_raw<B>(&self, path: &str, body: B, content_type: &str) -> Result<Response>
    where
        reqwest::Body: From<B>,
    {
        let request = self
            .prepare_request_headers(Method::POST, path, &[], content_type)
            .await?
            .body(body);
        let res = self.send_request(request).await?;
        Ok(res)
    }

    pub async fn post_json<B, T>(
        &self, path: &str, params: &[(&str, &str)], body: B,
    ) -> Result<T, anyhow::Error>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let response = self.post(path, params, body).await?.error_for_status()?;
        let parsed = response.json::<T>().await?;
        Ok(parsed)
    }

    async fn prepare_request(
        &self, method: Method, path: &str, params: &[(&str, &str)],
    ) -> Result<RequestBuilder> {
        let (mut url, headers) = self.get_url_and_headers().await?;
        url = url.join(path)?;
        self.add_params_to_url(&mut url, params);
        Ok(self.client.request(method, url).headers(headers))
    }

    async fn prepare_request_headers(
        &self, method: Method, path: &str, params: &[(&str, &str)], content_type: &str,
    ) -> Result<RequestBuilder> {
        let (mut url, mut headers) = self.get_url_and_headers().await?;
        url = url.join(path)?;
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_str(content_type)?,
        );
        self.add_params_to_url(&mut url, params);
        Ok(self.client.request(method, url).headers(headers))
    }

    async fn send_request(&self, request: RequestBuilder) -> Result<Response> {
        let permit = self.semaphore.acquire().await?;
        let res = match request.send().await {
            Ok(r) => r,
            Err(e) => return Err(anyhow!(e.to_user_facing())),
        };
        drop(permit);
        Ok(res)
    }

    pub async fn authenticate_admin(&self) -> Result<AuthenticateResponse> {
        let path = format!("Users/{}", self.user_id().await);
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
        debug!("Request URL: {}", url);
    }

    // jellyfin
    pub async fn get_direct_stream_url(
        &self, continer: &str, media_source_id: &str, etag: &str,
    ) -> String {
        let mut url = self.url.lock().await.as_ref().unwrap().to_owned();
        url.path_segments_mut().unwrap().pop();
        let path = format!("Videos/{media_source_id}/stream.{continer}");
        let mut url = url.join(&path).unwrap();
        url.query_pairs_mut()
            .append_pair("Static", "true")
            .append_pair("mediaSourceId", media_source_id)
            .append_pair("deviceId", &DEVICE_ID)
            .append_pair("api_key", self.user_access_token.lock().await.as_str())
            .append_pair("Tag", etag);
        url.to_string()
    }

    pub async fn search(
        &self, query: &str, filter: &[&str], start_index: &str, filters_list: &FiltersList,
    ) -> Result<List> {
        let filter_str = filter.join(",");
        let path = format!("Users/{}/Items", self.user_id().await);
        let mut params = vec![
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

        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });

        self.request(&path, &params).await
    }

    pub async fn get_episodes(&self, id: &str, season_id: &str, start_index: u32) -> Result<List> {
        let path = format!("Shows/{id}/Episodes");
        let params = [
            (
                "Fields",
                "Overview,PrimaryImageAspectRatio,PremiereDate,ProductionYear,SyncStatus",
            ),
            ("Limit", "50"),
            ("StartIndex", &start_index.to_string()),
            ("ImageTypeLimit", "1"),
            ("SeasonId", season_id),
            ("UserId", &self.user_id().await),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_episodes_all(&self, id: &str, season_id: &str) -> Result<List> {
        let path = format!("Shows/{id}/Episodes");
        let params = [
            (
                "Fields",
                "Overview,PrimaryImageAspectRatio,PremiereDate,ProductionYear,SyncStatus",
            ),
            ("ImageTypeLimit", "1"),
            ("SeasonId", season_id),
            ("UserId", &self.user_id().await),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_item_info(&self, id: &str) -> Result<SimpleListItem> {
        let path = format!("Users/{}/Items/{}", self.user_id().await, id);
        let params = [("Fields", "ShareLevel")];
        self.request(&path, &params).await
    }

    pub async fn get_edit_info(&self, id: &str) -> Result<Value> {
        let path = format!("Users/{}/Items/{}", self.user_id().await, id);
        let params = [("Fields", "ChannelMappingInfo")];
        self.request(&path, &params).await
    }

    pub async fn post_item(&self, id: &str, body: Value) -> Result<Response> {
        let path = format!("Items/{id}");
        self.post(&path, &[], body).await
    }

    pub async fn get_resume(&self) -> Result<List> {
        let path = format!("Users/{}/Items/Resume", self.user_id().await);
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
        let path = format!("Items/{id}/Images");
        self.request(&path, &[]).await
    }

    pub async fn image_request(
        &self, id: &str, image_type: &str, tag: Option<u8>, etag: Option<String>,
    ) -> Result<Response> {
        let mut path = format!("Items/{id}/Images/{image_type}");
        if let Some(tag) = tag {
            path.push_str(&format!("/{tag}"));
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
        self.request_picture(&path, &params, etag).await
    }

    pub async fn get_image(&self, id: &str, image_type: &str, tag: Option<u8>) -> Result<String> {
        let mut path = jellyfin_cache_path().await;
        path.push(format!("{}-{}-{}", id, image_type, tag.unwrap_or(0)));

        let mut etag: Option<String> = None;

        if path.exists() {
            #[cfg(target_os = "linux")]
            {
                etag = xattr::get(&path, "user.etag")
                    .ok()
                    .flatten()
                    .and_then(|v| String::from_utf8(v).ok());
            }
            #[cfg(target_os = "windows")]
            {
                etag = xattr::get_xattr(&path, "user.etag").ok();
            }
        }

        match self.image_request(id, image_type, tag, etag).await {
            Ok(response) => {
                if response.status().is_redirection() {
                    return Ok(path.to_string_lossy().to_string());
                } else if !response.status().is_success() {
                    return Err(anyhow!("Failed to get image: {}", response.status()));
                }

                let etag = response
                    .headers()
                    .get("ETag")
                    .map(|v| v.to_str().unwrap_or_default().to_string());

                let bytes = response.bytes().await?;

                let path = if bytes.len() > 1000 {
                    self.save_image(id, image_type, tag, &bytes, etag).await
                } else {
                    String::new()
                };

                Ok(path)
            }
            Err(e) => Err(e),
        }
    }

    // Only support base64 encoded images
    pub async fn post_image<B>(
        &self, id: &str, image_type: &str, bytes: B, content_type: &str,
    ) -> Result<Response>
    where
        reqwest::Body: From<B>,
    {
        let path = format!("Items/{id}/Images/{image_type}");
        self.post_raw(&path, bytes, content_type)
            .await?
            .error_for_status()
            .map_err(|e| e.into())
    }

    pub async fn post_image_url(
        &self, id: &str, image_type: &str, tag: u8, url: &str,
    ) -> Result<Response> {
        let path = format!("Items/{id}/Images/{tag}/{image_type}");
        let body = json!({ "Url": url });
        self.post(&path, &[], body).await
    }

    pub async fn delete_image(
        &self, id: &str, image_type: &str, tag: Option<u8>,
    ) -> Result<Response> {
        let mut path = format!("Items/{id}/Images/{image_type}");
        if let Some(tag) = tag {
            path.push_str(&format!("/{tag}"));
        }
        path.push_str("/Delete");
        self.post(&path, &[], json!({})).await
    }

    pub async fn save_image(
        &self, id: &str, image_type: &str, tag: Option<u8>, bytes: &[u8], etag: Option<String>,
    ) -> String {
        let cache_path = jellyfin_cache_path().await;
        let path = format!("{}-{}-{}", id, image_type, tag.unwrap_or(0));
        let path = cache_path.join(path);
        std::fs::write(&path, bytes).unwrap();
        if let Some(etag) = etag {
            #[cfg(target_os = "linux")]
            xattr::set(&path, "user.etag", etag.as_bytes()).unwrap_or_else(|e| {
                tracing::warn!("Failed to set etag xattr: {}", e);
            });
            #[cfg(target_os = "windows")]
            xattr::set_xattr(&path, "user.etag", etag).unwrap_or_else(|e| {
                tracing::warn!("Failed to set etag xattr: {}", e);
            });
        }
        path.to_string_lossy().to_string()
    }

    pub async fn get_artist_albums(&self, id: &str, artist_id: &str) -> Result<List> {
        let path = format!("Users/{}/Items", self.user_id().await);
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
            ("UserId", &self.user_id().await),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_playbackinfo(
        &self, id: &str, sub_stream_index: Option<u64>, media_source_id: Option<String>,
        is_playback: bool,
    ) -> Result<Media> {
        let path = format!("Items/{id}/PlaybackInfo");
        let subtitle_stream_index = sub_stream_index.map(|s| s.to_string()).unwrap_or_default();
        let params = [
            ("StartTimeTicks", "0"),
            ("UserId", &self.user_id().await),
            ("AutoOpenLiveStream", "true"),
            ("IsPlayback", &is_playback.to_string()),
            ("MediaSourceId", &media_source_id.unwrap_or_default()),
            ("SubtitleStreamIndex", &subtitle_stream_index),
            ("MaxStreamingBitrate", "2147483647"),
        ];
        let profile: Value = serde_json::from_str(PROFILE).expect("Failed to parse profile");
        self.post_json(&path, &params, profile).await
    }
    pub async fn scan(&self, id: &str) -> Result<Response> {
        let path = format!("Items/{id}/Refresh");
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
        &self, id: &str, replace_images: &str, replace_metadata: &str,
    ) -> Result<Response> {
        let path = format!("Items/{id}/Refresh");
        let params = [
            ("Recursive", "true"),
            ("ImageRefreshMode", "FullRefresh"),
            ("MetadataRefreshMode", "FullRefresh"),
            ("ReplaceAllImages", replace_images),
            ("ReplaceAllMetadata", replace_metadata),
        ];
        self.post(&path, &params, json!({})).await
    }

    pub async fn remote_search(&self, type_: &str, info: &RemoteSearchInfo) -> Result<Value> {
        let path = format!("Items/RemoteSearch/{type_}");
        let body = json!(info);
        self.post_json(&path, &[], body).await
    }

    pub async fn apply_remote_search(
        &self, id: &str, value: Value, replace_all_images: bool,
    ) -> Result<Response> {
        let path = format!("Items/RemoteSearch/Apply/{id}");
        let params: [(&str, &str); 1] = [("ReplaceAllImages", &replace_all_images.to_string())];
        self.post(&path, &params, json! {value}).await
    }

    pub async fn get_user_avatar(&self) -> Result<String> {
        let path = format!("Users/{}/Images/Primary", self.user_id().await);
        let params = [("maxHeight", "50"), ("maxWidth", "50")];
        let response = self.request_picture(&path, &params, None).await?;
        let etag = response
            .headers()
            .get("ETag")
            .map(|v| v.to_str().unwrap_or_default().to_string());
        let bytes = response.bytes().await?;
        let path = self
            .save_image(&self.user_id().await, "Primary", None, &bytes, etag)
            .await;
        Ok(path)
    }

    pub async fn get_external_id_info(&self, id: &str) -> Result<Vec<ExternalIdInfo>> {
        let path = format!("Items/{id}/ExternalIdInfos");
        let params = [("IsSupportedAsIdentifier", "true")];
        self.request(&path, &params).await
    }

    pub async fn get_library(&self) -> Result<List> {
        let path = format!("Users/{}/Views", &self.user_id().await);
        self.request(&path, &[]).await
    }

    pub async fn get_latest(&self, id: &str) -> Result<Vec<SimpleListItem>> {
        let path = format!("Users/{}/Items/Latest", &self.user_id().await);
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

    pub async fn get_streaming_url(&self, path: &str) -> String {
        let url = self.url.lock().await.as_ref().unwrap().to_owned();
        url.join(path.trim_start_matches('/')).unwrap().to_string()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_list(
        &self, id: &str, start: u32, include_item_types: &str, list_type: ListType,
        sort_order: &str, sortby: &str, filters_list: &FiltersList,
    ) -> Result<List> {
        let user_id = &self.user_id().await;
        let path = match list_type {
            ListType::All => format!("Users/{user_id}/Items"),
            ListType::Resume => format!("Users/{user_id}/Items/Resume"),
            ListType::Genres => "Genres".to_string(),
            _ => format!("Users/{user_id}/Items"),
        };
        let include_item_type = match list_type {
            ListType::Tags => "Tag",
            ListType::BoxSet => "BoxSet",
            _ => include_item_types,
        };
        let start_string = start.to_string();
        let mut params = match list_type {
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
                    if list_type == ListType::Liked {
                        ("Filters", "IsFavorite")
                    } else {
                        ("", "")
                    },
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
        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });
        self.request(&path, &params).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_inlist(
        &self, id: Option<String>, start: u32, listtype: &str, parentid: &str, sort_order: &str,
        sortby: &str, filters_list: &FiltersList,
    ) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id().await);
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
            if listtype == "Genres" || listtype == "Genre" {
                ("GenreIds", parentid)
            } else if listtype == "Studios" {
                ("StudioIds", parentid)
            } else {
                ("TagIds", parentid)
            },
        ];
        let id_clone;
        if let Some(id) = id {
            id_clone = id.to_owned();
            params.push(("ParentId", &id_clone));
        }

        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });
        self.request(&path, &params).await
    }

    pub async fn like(&self, id: &str) -> Result<()> {
        let path = format!("Users/{}/FavoriteItems/{}", &self.user_id().await, id);
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn unlike(&self, id: &str) -> Result<()> {
        let path = format!(
            "Users/{}/FavoriteItems/{}/Delete",
            &self.user_id().await,
            id
        );
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn set_as_played(&self, id: &str) -> Result<()> {
        let path = format!("Users/{}/PlayedItems/{}", &self.user_id().await, id);
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn set_as_unplayed(&self, id: &str) -> Result<()> {
        let path = format!("Users/{}/PlayedItems/{}/Delete", &self.user_id().await, id);
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
        let body = json!({"VolumeLevel":100,"NowPlayingQueue":[],"IsMuted":false,"IsPaused":false,"MaxStreamingBitrate":2147483647,"RepeatMode":"RepeatNone","PlaybackStartTimeTicks":back.start_tick,"SubtitleOffset":0,"PlaybackRate":1,"PositionTicks":back.tick,"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"PlaylistIndex":0,"PlaylistLength":1,"CanSeek":true,"ItemId":back.id,"Shuffle":false});
        self.post(&path, &params, body).await?;
        Ok(())
    }

    pub async fn get_similar(&self, id: &str) -> Result<List> {
        let path = format!("Items/{id}/Similar");
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("UserId", &self.user_id().await),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_actor_item_list(&self, id: &str, types: &str) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id().await);
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
        &self, id: &str, types: &str, sort_by: &str, sort_order: &str, start_index: u32,
        filters_list: &FiltersList,
    ) -> Result<List> {
        let start_string = start_index.to_string();
        let path = format!("Users/{}/Items", &self.user_id().await);
        let mut params = vec![
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

        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });

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
            ("UserId", &self.user_id().await),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_season_list(&self, parent_id: &str) -> Result<List> {
        let path = format!("Shows/{parent_id}/Seasons");
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PremiereDate,PrimaryImageAspectRatio,Overview",
            ),
            ("UserId", &self.user_id().await),
            ("ImageTypeLimit", "1"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_search_recommend(&self) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id().await);
        let params = [
            ("Limit", "20"),
            ("Fields", "Overview"),
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
        &self, types: &str, start: u32, limit: u32, sort_by: &str, sort_order: &str,
        filters_list: &FiltersList,
    ) -> Result<List> {
        let user_id = self.user_id.lock().await;
        let path = if types == "People" {
            "Persons".to_string()
        } else {
            format!("Users/{user_id}/Items")
        };
        let limit_string = limit.to_string();
        let start_string = start.to_string();
        let mut params = vec![
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
            ("Limit", &limit_string),
            ("StartIndex", &start_string),
            if types == "People" {
                ("UserId", &user_id)
            } else {
                ("", "")
            },
        ];

        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });

        self.request(&path, &params).await
    }

    pub async fn get_included(&self, id: &str) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id().await);
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
        let path = format!("Users/{}/Items", &self.user_id().await);
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
        ];
        self.request(&path, &params).await
    }

    pub async fn get_folder_include(
        &self, parent_id: &str, sort_by: &str, sort_order: &str, start_index: u32,
        filters_list: &FiltersList,
    ) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id().await);
        let start_index_string = start_index.to_string();
        let sort_by = format!("IsFolder,{sort_by}");
        let mut params = vec![
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("StartIndex", &start_index_string),
            ("ImageTypeLimit", "1"),
            ("Limit", "50"),
            ("ParentId", parent_id),
            ("SortBy", &sort_by),
            ("SortOrder", sort_order),
            ("EnableTotalRecordCount", "true"),
        ];

        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });
        self.request(&path, &params).await
    }

    pub async fn change_password(&self, new_password: &str) -> Result<()> {
        let path = format!("Users/{}/Password", &self.user_id().await);

        let old_password = self.user_password.lock().await.to_owned();

        let body = json!({
            "CurrentPw": old_password,
            "NewPw": new_password
        });

        self.post(&path, &[], body).await?;
        Ok(())
    }

    pub async fn hide_from_resume(&self, id: &str) -> Result<()> {
        let path = format!(
            "Users/{}/Items/{}/HideFromResume",
            &self.user_id().await,
            id
        );
        let params = [("Hide", "true")];
        self.post(&path, &params, json!({})).await?;
        Ok(())
    }

    pub async fn get_songs(&self, parent_id: &str) -> Result<List> {
        let path = format!("Users/{}/Items", &self.user_id().await);
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

    pub async fn get_song_streaming_uri(&self, id: &str) -> String {
        let url = self.url.lock().await.as_ref().unwrap().to_owned();

        url.join(&format!("Audio/{}/universal?UserId={}&DeviceId={}&MaxStreamingBitrate=4000000&Container=opus,mp3|mp3,mp2,mp3|mp2,m4a|aac,mp4|aac,flac,webma,webm,wav|PCM_S16LE,wav|PCM_S24LE,ogg&TranscodingContainer=aac&TranscodingProtocol=hls&AudioCodec=aac&api_key={}&PlaySessionId=1715006733496&StartTimeTicks=0&EnableRedirection=true&EnableRemoteMedia=false",
        id, &self.user_id().await, &DEVICE_ID.to_string(), self.user_access_token.lock().await, )).unwrap().to_string()
    }

    async fn user_id(&self) -> String {
        self.user_id.lock().await.to_string()
    }

    pub async fn get_additional(&self, id: &str) -> Result<List> {
        let path = format!("Videos/{id}/AdditionalParts");
        let params: [(&str, &str); 1] = [("UserId", &self.user_id().await)];
        self.request(&path, &params).await
    }

    pub async fn get_channels(&self) -> Result<List> {
        let params = [
            ("IsAiring", "true"),
            ("userId", &self.user_id().await),
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
            ("userId", &self.user_id().await),
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

    pub async fn get_image_path(
        &self, id: &str, image_type: &str, image_index: Option<u32>,
    ) -> String {
        let path = format!("Items/{id}/Images/{image_type}/");
        let url = self
            .url
            .lock()
            .await
            .as_ref()
            .unwrap()
            .to_owned()
            .join(&path)
            .unwrap();
        match image_index {
            Some(index) => url.join(&index.to_string()).unwrap().to_string(),
            None => url.to_string(),
        }
    }

    pub async fn get_remote_image_list(
        &self, id: &str, start_index: u32, include_all_languages: bool, type_: &str,
        provider_name: &str,
    ) -> Result<ImageSearchResult> {
        let path = format!("Items/{id}/RemoteImages");
        let start_string = start_index.to_string();
        let params = [
            ("Limit", "50"),
            ("StartIndex", &start_string),
            ("Type", type_),
            ("IncludeAllLanguages", &include_all_languages.to_string()),
            ("ProviderName", provider_name),
        ];

        self.request(&path, &params).await
    }

    pub async fn delete_info(&self, id: &str) -> Result<DeleteInfo> {
        let path = format!("Items/{id}/DeleteInfo");
        self.request(&path, &[]).await
    }

    pub async fn delete(&self, ids: &str) -> Result<Response> {
        let params = [("Ids", ids)];
        self.post("Items/Delete", &params, json!({})).await
    }

    pub async fn download_remote_images(
        &self, id: &str, type_: &str, provider_name: &str, image_url: &str,
    ) -> Result<()> {
        let path = format!("Items/{id}/RemoteImages/Download");
        let params = [
            ("Type", type_),
            ("ProviderName", provider_name),
            ("ImageUrl", image_url),
        ];
        self.post(&path, &params, json!({})).await?;
        Ok(())
    }

    pub async fn get_show_missing(
        &self, id: &str, include_specials: bool, upcoming: bool,
    ) -> Result<MissingEpisodesList> {
        let params = [
            ("Fields", "Overview"),
            ("UserId", &self.user_id().await),
            ("ParentId", id),
            ("IncludeSpecials", &include_specials.to_string()),
            ("IncludeUnaired", &upcoming.to_string()),
        ];
        self.request("Shows/Missing", &params).await
    }

    pub async fn reset_metadata(&self, ids: &str) -> Result<Response> {
        self.post("items/metadata/reset", &[], json!({"Ids": ids}))
            .await
    }

    pub async fn filters(&self, type_: &str) -> Result<FilterList> {
        let params = [
            ("SortBy", "SortName"),
            ("SortOrder", "Ascending"),
            ("Recursive", "true"),
            ("EnableImages", "false"),
            ("EnableUserData", "false"),
            (
                "IncludeItemTypes",
                "Movie,Series,Episode,BoxSet,Person,MusicAlbum,Audio,Video",
            ),
            ("userId", &self.user_id().await),
        ];
        self.request(type_, &params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::error::UserFacingError;

    #[tokio::test]
    async fn search() {
        let _ = JELLYFIN_CLIENT
            .header_change_url("https://example.com", "443")
            .await;
        let result = JELLYFIN_CLIENT.login("test", "test").await;
        match result {
            Ok(response) => {
                println!("{}", response.access_token);
                let _ = JELLYFIN_CLIENT
                    .header_change_token(&response.access_token)
                    .await;
                let _ = JELLYFIN_CLIENT.set_user_id(&response.user.id).await;
            }
            Err(e) => {
                eprintln!("{}", e.to_user_facing());
            }
        }
        let filters_list = FiltersList::default();
        let result = JELLYFIN_CLIENT.search("你的名字", &["Movie"], "0", &filters_list);
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
            format!("http://{uri}")
        } else {
            uri.to_string()
        };

        assert_eq!(url, "http://127.0.0.1");
    }

    #[tokio::test]
    async fn test_upload_image() {
        let _ = JELLYFIN_CLIENT
            .header_change_url("http://127.0.0.1", "8096")
            .await;
        let result = JELLYFIN_CLIENT.login("inaha", "").await;
        match result {
            Ok(response) => {
                println!("{}", response.access_token);
                let account = Account {
                    servername: "test".to_string(),
                    server: "http://127.0.0.1".to_string(),
                    username: "inaha".to_string(),
                    password: String::new(),
                    port: "8096".to_string(),
                    user_id: response.user.id,
                    access_token: response.access_token,
                    server_type: Some("Jellyfin".to_string()),
                };
                let _ = JELLYFIN_CLIENT.init(&account).await;
            }
            Err(e) => {
                eprintln!("{}", e.to_user_facing());
            }
        }

        let image = std::fs::read("/home/inaha/Works/tsukimi/target/debug/test.jpg").unwrap();
        use base64::{
            Engine as _,
            engine::general_purpose::STANDARD,
        };
        let image = STANDARD.encode(&image);
        match JELLYFIN_CLIENT
            .post_image("293", "Thumb", image, "image/jpeg")
            .await
        {
            Ok(_) => {
                println!("success");
            }
            Err(e) => {
                eprintln!("{}", e.to_user_facing());
            }
        }
    }
}
