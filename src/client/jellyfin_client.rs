use std::{
    cmp::Reverse,
    collections::HashMap,
    future,
    hash::Hasher,
    time::Duration,
};

use crate::{
    client::account::ServerType,
    ui::PlaybackDirectMode,
};
use anyhow::{
    Context,
    Result,
    anyhow,
    bail,
};
use arc_swap::ArcSwap;
use chrono::{
    DateTime,
    Utc,
};
use futures_util::{
    StreamExt,
    stream::FuturesUnordered,
};
use moka::future::Cache;
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
use std::sync::Arc;
use tracing::warn;
use url::Url;
use uuid::Uuid;

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
        MediaSegmentList,
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
    client::cache_metadata,
    config::version,
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

#[derive(Clone)]
pub struct Session {
    pub account: Account,
    pub url: Option<Url>,
    pub headers: reqwest::header::HeaderMap,
    pub server_name_hash: String,
}

impl Session {
    fn empty() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));
        headers.insert(
            "X-Emby-authorization",
            HeaderValue::from_str(&generate_jellyfin_authorization(
                "",
                CLIENT_ID,
                &DEVICE_NAME,
                &DEVICE_ID,
                version(),
            ))
            .unwrap(),
        );
        Self {
            account: Account::default(),
            url: None,
            headers,
            server_name_hash: String::new(),
        }
    }
}

pub struct JellyfinClient {
    pub session: ArcSwap<Session>,
    pub semaphore: tokio::sync::Semaphore,
    pub client: Client,
    next_up_date_cache: Cache<NextUpDateKey, Option<DateTime<Utc>>>,
}

#[derive(Hash, PartialEq, Eq)]
struct NextUpDateKey {
    series_id: String,
    item_id: String,
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
        Self {
            session: ArcSwap::from_pointee(Session::empty()),
            semaphore: tokio::sync::Semaphore::new(SETTINGS.threads() as usize),
            client: ReqClient::build(),
            next_up_date_cache: Cache::builder()
                .max_capacity(256)
                .time_to_live(Duration::from_hours(2))
                .support_invalidation_closures()
                .build(),
        }
    }
}

impl JellyfinClient {
    pub fn session(&self) -> arc_swap::Guard<Arc<Session>> {
        self.session.load()
    }

    fn server_type(&self) -> ServerType {
        self.session.load().account.server_type.unwrap_or_default()
    }

    pub fn is_jellyfin(&self) -> bool {
        matches!(self.server_type(), ServerType::Jellyfin)
    }

    pub async fn init(&self, account: &Account) -> Result<(), Box<dyn std::error::Error>> {
        let url = {
            let mut url = Url::parse(&account.server)?;
            url.set_port(Some(account.port.parse::<u16>().unwrap_or_default()))
                .map_err(|_| anyhow!("Failed to set port"))?;
            url.join("emby/")?
        };

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));
        headers.insert(
            "X-Emby-Token",
            HeaderValue::from_str(&account.access_token)?,
        );
        headers.insert(
            "X-Emby-authorization",
            HeaderValue::from_str(&generate_jellyfin_authorization(
                &account.user_id,
                CLIENT_ID,
                &DEVICE_NAME,
                &DEVICE_ID,
                version(),
            ))?,
        );

        self.session.store(Arc::new(Session {
            account: account.clone(),
            url: Some(url),
            headers,
            server_name_hash: generate_hash(&account.servername),
        }));
        self.next_up_date_cache.invalidate_all();

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

    pub fn header_change_url(&self, url_str: &str, port: &str) -> Result<()> {
        let mut url = Url::parse(url_str)?;
        url.set_port(Some(port.parse::<u16>().unwrap_or_default()))
            .map_err(|_| anyhow!("Failed to set port"))?;
        let url = url.join("emby/")?;
        self.session.rcu(|current| {
            let mut session = (**current).clone();
            session.url = Some(url.clone());
            Arc::new(session)
        });
        Ok(())
    }

    pub fn header_change_token(&self, token: &str) -> Result<()> {
        let token = HeaderValue::from_str(token)?;
        self.session.rcu(|current| {
            let mut session = (**current).clone();
            session.headers.insert("X-Emby-Token", token.clone());
            Arc::new(session)
        });
        Ok(())
    }

    pub async fn request<T>(&self, path: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let request = self.prepare_request(Method::GET, path, params)?;
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
            .prepare_request(Method::GET, path, params)?
            .header("If-None-Match", etag.unwrap_or_default());
        let res = request.send().await?;
        Ok(res)
    }

    pub async fn delete(&self, path: &str, params: &[(&str, &str)]) -> Result<Response> {
        let request = self.prepare_request(Method::DELETE, path, params)?;
        self.send_request(request).await
    }

    pub async fn post<B>(&self, path: &str, params: &[(&str, &str)], body: B) -> Result<Response>
    where
        B: Serialize,
    {
        let request = self
            .prepare_request(Method::POST, path, params)?
            .json(&body);
        self.send_request(request).await
    }

    pub async fn post_raw<B>(&self, path: &str, body: B, content_type: &str) -> Result<Response>
    where
        reqwest::Body: From<B>,
    {
        let request = self
            .prepare_request_headers(Method::POST, path, &[], content_type)?
            .body(body);
        self.send_request(request).await
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

    fn prepare_request(
        &self, method: Method, path: &str, params: &[(&str, &str)],
    ) -> Result<RequestBuilder> {
        let s = self.session();
        let url = s.url.as_ref().context("URL is not set")?.join(path)?;
        Ok(self
            .client
            .request(method, url)
            .query(params)
            .headers(s.headers.clone()))
    }

    fn prepare_request_headers(
        &self, method: Method, path: &str, params: &[(&str, &str)], content_type: &str,
    ) -> Result<RequestBuilder> {
        let s = self.session();
        let url = s.url.as_ref().context("URL is not set")?.join(path)?;
        let mut headers = s.headers.clone();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_str(content_type)?,
        );
        Ok(self
            .client
            .request(method, url)
            .query(params)
            .headers(headers))
    }

    async fn send_request(&self, request: RequestBuilder) -> Result<Response> {
        let _permit = self.semaphore.acquire().await?;
        request
            .send()
            .await
            .map_err(|e| anyhow!(e.to_user_facing()))
    }

    pub async fn authenticate_admin(&self) -> Result<AuthenticateResponse> {
        let s = self.session();
        let path = format!("Users/{}", s.account.user_id);
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

    pub async fn get_item_stream_url(
        &self, container: &str, item_id: &str, media_source_id: &str,
    ) -> Result<String> {
        let s = self.session();
        let Some(url) = s.url.as_ref() else {
            bail!("URL is not set");
        };
        let path = format!("Videos/{}/stream.{}", item_id, container);
        let mut url = url.join(&path).context("Failed to build item stream URL")?;
        url.query_pairs_mut()
            .append_pair("Static", "true")
            .append_pair("deviceId", &DEVICE_ID)
            .append_pair("api_key", &s.account.access_token)
            .append_pair("MediaSourceId", media_source_id);
        Ok(url.to_string())
    }

    pub async fn search(
        &self, query: &str, filter: &[&str], start_index: &str, filters_list: &FiltersList,
    ) -> Result<List> {
        let s = self.session();
        let filter_str = filter.join(",");
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
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
            ("UserId", &s.account.user_id),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_episodes_all(&self, id: &str, season_id: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Shows/{id}/Episodes");
        let params = [
            (
                "Fields",
                "Overview,PrimaryImageAspectRatio,PremiereDate,ProductionYear,SyncStatus",
            ),
            ("ImageTypeLimit", "1"),
            ("SeasonId", season_id),
            ("UserId", &s.account.user_id),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_item_info(&self, id: &str) -> Result<SimpleListItem> {
        let s = self.session();
        let path = format!("Users/{}/Items/{}", s.account.user_id, id);
        let params = [("Fields", "ShareLevel")];
        self.request(&path, &params).await
    }

    pub async fn get_edit_info(&self, id: &str) -> Result<Value> {
        let s = self.session();
        let path = format!("Users/{}/Items/{}", s.account.user_id, id);
        let params = [("Fields", "ChannelMappingInfo")];
        self.request(&path, &params).await
    }

    pub async fn post_item(&self, id: &str, body: Value) -> Result<Response> {
        let path = format!("Items/{id}");
        self.post(&path, &[], body).await
    }

    pub async fn get_resume(&self, limit: u32) -> Result<List> {
        let s = self.session();
        let path = format!("Users/{}/Items/Resume", s.account.user_id);
        let params = [
            ("Recursive", "true"),
            (
                "Fields",
                "Overview,BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,CommunityRating",
            ),
            ("EnableImageTypes", "Primary,Backdrop,Thumb,Banner"),
            ("ImageTypeLimit", "1"),
            ("MediaTypes", "Video"),
            ("Limit", &limit.to_string()),
        ];
        self.request(&path, &params).await
    }

    pub fn next_up_date_cutoff(&self) -> String {
        (chrono::Utc::now() - chrono::Duration::days(365))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    pub async fn get_next_up(&self, limit: u32, next_up_date_cutoff: &str) -> Result<List> {
        if !self.is_jellyfin() {
            bail!("Next up is not supported on Emby");
        }
        let s = self.session();
        let limit = limit.to_string();
        let params = [
            ("Limit", limit.as_str()),
            (
                "Fields",
                "PrimaryImageAspectRatio,DateCreated,Path,MediaSourceCount",
            ),
            ("UserId", &s.account.user_id),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary,Backdrop,Banner,Thumb"),
            ("EnableTotalRecordCount", "false"),
            ("DisableFirstEpisode", "true"),
            ("NextUpDateCutoff", next_up_date_cutoff),
            ("EnableResumable", "false"),
            ("EnableRewatching", "false"),
        ];
        self.request("Shows/NextUp", &params).await
    }

    /// Get next up and resume items, merge them and sort by last played date in client side.
    pub async fn get_next_up_merged(&self, limit: u32, next_up_date_cutoff: &str) -> Result<List> {
        if !self.is_jellyfin() {
            bail!("Next up is not supported on Emby");
        }
        let (resume, next_up) = tokio::try_join!(
            self.get_resume(limit),
            self.get_next_up(limit, next_up_date_cutoff)
        )?;

        let date_futures = next_up
            .items
            .iter()
            .map(|item| async move { (item.id.clone(), self.get_next_up_date(item).await) })
            .collect::<FuturesUnordered<_>>();

        let next_up_dates = date_futures
            .filter_map(|(id, date)| future::ready(date.map(|d| (id, d))))
            .collect::<HashMap<_, _>>()
            .await;

        let mut items = resume.items;
        items.extend(next_up.items);
        items.sort_by_key(|item| {
            Reverse(
                next_up_dates
                    .get(&item.id)
                    .copied()
                    .or_else(|| {
                        item.user_data
                            .as_ref()
                            .and_then(|user_data| user_data.last_played_date)
                    })
                    .unwrap_or(DateTime::<Utc>::MIN_UTC),
            )
        });
        items.truncate(limit as usize);

        Ok(List {
            total_record_count: items.len() as u32,
            items,
        })
    }

    async fn get_next_up_date(&self, item: &SimpleListItem) -> Option<DateTime<Utc>> {
        let series_id = item.series_id.as_ref()?;
        let user_id = &self.session().account.user_id;
        let item_id = &item.id;
        let key = NextUpDateKey {
            series_id: series_id.to_owned(),
            item_id: item_id.to_owned(),
        };
        self.next_up_date_cache
            .get_with(key, async move {
                let path = format!("Shows/{series_id}/Episodes");
                let params = [
                    ("UserId", user_id.as_str()),
                    ("AdjacentTo", item_id.as_str()),
                    ("Limit", "1"),
                ];
                let list: Result<List> = self.request(&path, &params).await;
                list.inspect_err(|e| {
                    warn!("Failed to get next up last played time: {}", e);
                })
                .ok()
                .and_then(|list| {
                    list.items.first().and_then(|episode| {
                        episode
                            .user_data
                            .as_ref()
                            .and_then(|user_data| user_data.last_played_date)
                    })
                })
            })
            .await
    }

    fn invalidate_next_up_date(&self, series_id: String) {
        let _ = self
            .next_up_date_cache
            .invalidate_entries_if(move |key, _| key.series_id == series_id);
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
            etag = cache_metadata::get_etag(&path);
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
        tokio::fs::write(&path, bytes).await.unwrap();
        if let Some(etag) = etag {
            cache_metadata::set_etag(&path, &etag).unwrap_or_else(|e| {
                tracing::warn!("Failed to set image cache etag: {}", e);
            });
        }
        path.to_string_lossy().to_string()
    }

    pub async fn get_artist_albums(&self, id: &str, artist_id: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        let path = "Shows/NextUp".to_string();
        let params = [
            ("Fields", "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio"),
            ("Limit", "1"),
            ("ImageTypeLimit", "1"),
            ("SeriesId", series_id),
            ("UserId", &s.account.user_id),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_playbackinfo(
        &self, id: &str, sub_stream_index: Option<i64>, media_source_id: Option<String>,
        is_playback: bool, direct_mode: PlaybackDirectMode,
    ) -> Result<Media> {
        let s = self.session();
        let path = format!("Items/{id}/PlaybackInfo");
        let subtitle_stream_index = sub_stream_index.map(|s| s.to_string()).unwrap_or_default();
        let params = [
            ("StartTimeTicks", "0"),
            ("UserId", &s.account.user_id),
            ("AutoOpenLiveStream", "true"),
            ("IsPlayback", &is_playback.to_string()),
            ("MediaSourceId", &media_source_id.unwrap_or_default()),
            ("SubtitleStreamIndex", &subtitle_stream_index),
            ("MaxStreamingBitrate", "2147483647"),
            (
                "EnableDirectPlay",
                &direct_mode.enable_direct_play.to_string(),
            ),
            (
                "EnableDirectStream",
                &direct_mode.enable_direct_stream.to_string(),
            ),
        ];
        let profile: Value = serde_json::from_str(PROFILE).expect("Failed to parse profile");
        self.post_json(&path, &params, profile).await
    }

    pub async fn get_skippable_segments(&self, id: &str) -> Result<MediaSegmentList> {
        if !self.is_jellyfin() {
            bail!("Skippable segments are not supported on Emby");
        }
        let path = format!("MediaSegments/{id}");
        let params = [
            ("includeSegmentTypes", "Intro"),
            ("includeSegmentTypes", "Outro"),
        ];
        self.request(&path, &params).await
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
        let s = self.session();
        let path = format!("Users/{}/Images/Primary", s.account.user_id);
        let params = [("maxHeight", "50"), ("maxWidth", "50")];
        let response = self.request_picture(&path, &params, None).await?;
        let etag = response
            .headers()
            .get("ETag")
            .map(|v| v.to_str().unwrap_or_default().to_string());
        let bytes = response.bytes().await?;
        let path = self
            .save_image(&s.account.user_id, "Primary", None, &bytes, etag)
            .await;
        Ok(path)
    }

    pub async fn get_external_id_info(&self, id: &str) -> Result<Vec<ExternalIdInfo>> {
        let path = format!("Items/{id}/ExternalIdInfos");
        let params = [("IsSupportedAsIdentifier", "true")];
        self.request(&path, &params).await
    }

    pub async fn get_library(&self) -> Result<List> {
        let s = self.session();
        let path = format!("Users/{}/Views", s.account.user_id);
        self.request(&path, &[]).await
    }

    pub async fn get_latest(&self, id: &str) -> Result<Vec<SimpleListItem>> {
        let s = self.session();
        let path = format!("Users/{}/Items/Latest", s.account.user_id);
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
        let s = self.session();
        let url = s.url.as_ref().expect("URL not set");
        url.join(path.trim_start_matches('/')).unwrap().to_string()
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_list(
        &self, id: &str, start: u32, include_item_types: &str, list_type: ListType,
        sort_order: &str, sortby: &str, filters_list: &FiltersList,
    ) -> Result<List> {
        let s = self.session();
        let user_id = &s.account.user_id;
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
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        if let Some(id) = id.as_deref() {
            params.push(("ParentId", id));
        }
        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });
        self.request(&path, &params).await
    }

    pub async fn like(&self, id: &str) -> Result<()> {
        let s = self.session();
        let path = format!("Users/{}/FavoriteItems/{}", s.account.user_id, id);
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn unlike(&self, id: &str) -> Result<()> {
        let s = self.session();
        match self.server_type() {
            ServerType::Emby => {
                let path = format!("Users/{}/FavoriteItems/{}/Delete", s.account.user_id, id);
                self.post(&path, &[], json!({})).await?;
            }
            ServerType::Jellyfin => {
                let path = format!("Users/{}/FavoriteItems/{}", s.account.user_id, id);
                self.delete(&path, &[]).await?;
            }
        }
        Ok(())
    }

    pub async fn set_as_played<T: Into<String>>(
        &self, id: &str, series_id: Option<T>,
    ) -> Result<()> {
        let s = self.session();
        let path = format!("Users/{}/PlayedItems/{}", s.account.user_id, id);
        self.post(&path, &[], json!({})).await?;
        if let Some(series_id) = series_id {
            self.invalidate_next_up_date(series_id.into());
        }
        Ok(())
    }

    pub async fn set_as_unplayed<T: Into<String>>(
        &self, id: &str, series_id: Option<T>,
    ) -> Result<()> {
        let s = self.session();
        match self.server_type() {
            ServerType::Emby => {
                let path = format!("Users/{}/PlayedItems/{}/Delete", s.account.user_id, id);
                self.post(&path, &[], json!({})).await?;
            }
            ServerType::Jellyfin => {
                let path = format!("Users/{}/PlayedItems/{}", s.account.user_id, id);
                self.delete(&path, &[]).await?;
            }
        }
        if let Some(series_id) = series_id {
            self.invalidate_next_up_date(series_id.into());
        }
        Ok(())
    }

    pub async fn position_back(&self, back: &Back, backtype: BackType) -> Result<()> {
        let path = match backtype {
            BackType::Start => "Sessions/Playing",
            BackType::Stop => "Sessions/Playing/Stopped",
            BackType::Back => "Sessions/Playing/Progress",
        };
        let params = [("reqformat", "json")];
        let body = json!({
            "VolumeLevel":100,
            "NowPlayingQueue":[],
            "IsMuted":false,
            "IsPaused":false,
            "MaxStreamingBitrate":2147483647,
            "RepeatMode":"RepeatNone",
            "PlaybackStartTimeTicks":back.start_tick,
            "SubtitleOffset":0,
            "PlaybackRate":1,
            "PositionTicks":back.tick,
            "PlayMethod":back.playmethod,
            "PlaySessionId":back.playsessionid,
            "LiveStreamId":back.livestreamid,
            "MediaSourceId":back.mediasourceid,
            "PlaylistIndex":0,
            "PlaylistLength":1,
            "CanSeek":true,
            "ItemId":back.id,
            "Shuffle":false
        });
        self.post(path, &params, body).await?;
        if matches!(backtype, BackType::Stop)
            && let Some(series_id) = back.series_id.as_deref()
        {
            self.invalidate_next_up_date(series_id.to_owned());
        }
        Ok(())
    }

    pub async fn get_similar(&self, id: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Items/{id}/Similar");
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("UserId", &s.account.user_id),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_actor_item_list(&self, id: &str, types: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        let start_string = start_index.to_string();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,Overview",
            ),
            ("Limit", "40"),
            ("ImageTypeLimit", "1"),
            ("SeriesId", parent_id),
            ("UserId", &s.account.user_id),
        ];
        self.request("Shows/NextUp", &params).await
    }

    pub async fn get_season_list(&self, parent_id: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Shows/{parent_id}/Seasons");
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PremiereDate,PrimaryImageAspectRatio,Overview",
            ),
            ("UserId", &s.account.user_id),
            ("ImageTypeLimit", "1"),
        ];
        self.request(&path, &params).await
    }

    pub async fn get_search_recommend(&self) -> Result<List> {
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        let user_id = &s.account.user_id;
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
        ];
        if types == "People" {
            params.push(("UserId", user_id));
        }
        let kv = filters_list.to_kv();
        kv.iter().for_each(|(k, v)| {
            params.push((k.as_str(), v.as_str()));
        });
        self.request(&path, &params).await
    }

    pub async fn get_included(&self, id: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        let path = format!("Users/{}/Password", s.account.user_id);
        let old_password = s.account.password.as_str();
        let body = json!({
            "CurrentPw": old_password,
            "NewPw": new_password
        });
        self.post(&path, &[], body).await?;
        Ok(())
    }

    pub async fn hide_from_resume<T: Into<String>>(
        &self, id: &str, series_id: Option<T>,
    ) -> Result<()> {
        let s = self.session();
        let path = format!("Users/{}/Items/{}/HideFromResume", s.account.user_id, id);
        let params = [("Hide", "true")];
        self.post(&path, &params, json!({})).await?;
        if let Some(series_id) = series_id {
            self.invalidate_next_up_date(series_id.into());
        }
        Ok(())
    }

    pub async fn get_songs(&self, parent_id: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Users/{}/Items", s.account.user_id);
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
        let s = self.session();
        s.url.as_ref().expect("URL not set").join(&format!("Audio/{}/universal?UserId={}&DeviceId={}&MaxStreamingBitrate=4000000&Container=opus,mp3|mp3,mp2,mp3|mp2,m4a|aac,mp4|aac,flac,webma,webm,wav|PCM_S16LE,wav|PCM_S24LE,ogg&TranscodingContainer=aac&TranscodingProtocol=hls&AudioCodec=aac&api_key={}&PlaySessionId=1715006733496&StartTimeTicks=0&EnableRedirection=true&EnableRemoteMedia=false",
        id, s.account.user_id, *DEVICE_ID, s.account.access_token, )).unwrap().to_string()
    }

    pub async fn get_additional(&self, id: &str) -> Result<List> {
        let s = self.session();
        let path = format!("Videos/{id}/AdditionalParts");
        let params: [(&str, &str); 1] = [("UserId", &s.account.user_id)];
        self.request(&path, &params).await
    }

    pub async fn get_channels(&self) -> Result<List> {
        let s = self.session();
        let params = [
            ("IsAiring", "true"),
            ("userId", &s.account.user_id),
            ("ImageTypeLimit", "1"),
            ("Limit", "12"),
            ("Fields", "ProgramPrimaryImageAspectRatio"),
            ("SortBy", "DefaultChannelOrder"),
            ("SortOrder", "Ascending"),
        ];
        self.request("LiveTv/Channels", &params).await
    }

    pub async fn get_channels_list(&self, start_index: u32) -> Result<List> {
        let s = self.session();
        let params = [
            ("IsAiring", "true"),
            ("userId", &s.account.user_id),
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
        let path = format!("ScheduledTasks/Running/{}", id);
        self.post(&path, &[], json!({})).await?;
        Ok(())
    }

    pub async fn get_image_path(
        &self, id: &str, image_type: &str, image_index: Option<u32>,
    ) -> String {
        let s = self.session();
        let path = format!("Items/{id}/Images/{image_type}/");
        let url = s.url.as_ref().expect("URL not set").join(&path).unwrap();
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

    pub async fn delete_item(&self, ids: &str) -> Result<Response> {
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
        let s = self.session();
        let params = [
            ("Fields", "Overview"),
            ("UserId", &s.account.user_id),
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
        let s = self.session();
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
            ("userId", &s.account.user_id),
        ];
        self.request(type_, &params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{
        account::ServerType,
        error::UserFacingError,
    };

    #[tokio::test]
    async fn search() {
        let _ = JELLYFIN_CLIENT.header_change_url("https://example.com", "443");
        let result = JELLYFIN_CLIENT.login("test", "test").await;
        match result {
            Ok(response) => {
                println!("{}", response.access_token);
                let account = Account {
                    servername: "test".to_string(),
                    server: "https://example.com".to_string(),
                    username: "inaha".to_string(),
                    password: String::new(),
                    port: "443".to_string(),
                    user_id: response.user.id,
                    access_token: response.access_token,
                    server_type: Some(ServerType::Jellyfin),
                };
                let _ = JELLYFIN_CLIENT.init(&account).await;
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
        let _ = JELLYFIN_CLIENT.header_change_url("http://127.0.0.1", "8096");
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
                    server_type: Some(ServerType::Jellyfin),
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
