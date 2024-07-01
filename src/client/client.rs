use std::sync::Mutex;

use reqwest::{header::HeaderValue, Method, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};
use url::Url;
use uuid::Uuid;

use crate::{
    config::{get_device_name, load_env, proxy::ReqClient, Account, APP_VERSION},
    ui::models::emby_cache_path,
    utils::{spawn, spawn_tokio},
};

use once_cell::sync::Lazy;

use super::structs::{
    AuthenticateResponse, Back, Item, List, LoginResponse, Media, SerInList, SimpleListItem,
};

pub static EMBY_CLIENT: Lazy<EmbyClient> = Lazy::new(EmbyClient::default);
pub static DEVICE_ID: Lazy<String> = Lazy::new(|| Uuid::new_v4().to_string());

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
        headers.insert("X-Emby-Client", HeaderValue::from_static("Tsukimi"));
        headers.insert(
            "X-Emby-Device-Name",
            HeaderValue::from_str(&get_device_name()).unwrap(),
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
        load_env(account);

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

    pub async fn search(&self, query: &str, filter: &[&str]) -> Result<List, reqwest::Error> {
        let filter_str = filter.join(",");
        let path = format!("Users/{}/Items", self.user_id());
        let params = [
            (
                "Fields",
                "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
            ),
            ("IncludeItemTypes", &filter_str),
            ("IncludeSearchTypes", &filter_str),
            ("StartIndex", "0"),
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
            ("Limit", "8"),
        ];
        self.request(&path, &params).await
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
                    "800"
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
                let bytes = response.bytes().await.unwrap();

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
            ("AutoOpenLiveStream", "false"),
            ("IsPlayback", "false"),
            ("AudioStreamIndex", "1"),
            ("SubtitleStreamIndex", "1"),
            ("MaxStreamingBitrate", "160000000"),
            ("reqformat", "json"),
        ];
        let body = json!(
            {"DeviceProfile":{"Name":"Direct play all","MaxStaticBitrate":1000000000,"MaxStreamingBitrate":1000000000,"MusicStreamingTranscodingBitrate":1500000,"DirectPlayProfiles":[{"Container":"mkv","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9,mp4","AudioCodec":"aac,ac3,alac,eac3,dts,flac,mp3,opus,truehd,vorbis"},{"Container":"mp4,m4v","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9","AudioCodec":"aac,alac,opus,mp3,flac,vorbis"},{"Container":"flv","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,mp3"},{"Container":"mov","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"opus","Type":"Audio"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3"},{"Container":"mp2,mp3","Type":"Audio","AudioCodec":"mp2"},{"Container":"m4a","AudioCodec":"aac","Type":"Audio"},{"Container":"mp4","AudioCodec":"aac","Type":"Audio"},{"Container":"flac","Type":"Audio"},{"Container":"webma,webm","Type":"Audio"},{"Container":"wav","Type":"Audio","AudioCodec":"PCM_S16LE,PCM_S24LE"},{"Container":"ogg","Type":"Audio"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis,opus","VideoCodec":"av1,VP8,VP9"}],"TranscodingProfiles":[],"ContainerProfiles":[],"CodecProfiles":[],"SubtitleProfiles":[{"Format":"vtt","Method":"Hls"},{"Format":"eia_608","Method":"VideoSideData","Protocol":"hls"},{"Format":"eia_708","Method":"VideoSideData","Protocol":"hls"},{"Format":"vtt","Method":"External"},{"Format":"ass","Method":"External"},{"Format":"ssa","Method":"External"}],"ResponseProfiles":[]}}
        );
        self.post(&path, &params, body).await?.json().await
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
        let body = json!(
            {"DeviceProfile":{"Name":"Direct play all","MaxStaticBitrate":1000000000,"MaxStreamingBitrate":1000000000,"MusicStreamingTranscodingBitrate":1500000,"DirectPlayProfiles":[{"Container":"mkv","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9,mp4","AudioCodec":"aac,ac3,alac,eac3,dts,flac,mp3,opus,truehd,vorbis"},{"Container":"mp4,m4v","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9","AudioCodec":"aac,alac,opus,mp3,flac,vorbis"},{"Container":"flv","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,mp3"},{"Container":"mov","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"opus","Type":"Audio"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3"},{"Container":"mp2,mp3","Type":"Audio","AudioCodec":"mp2"},{"Container":"m4a","AudioCodec":"aac","Type":"Audio"},{"Container":"mp4","AudioCodec":"aac","Type":"Audio"},{"Container":"flac","Type":"Audio"},{"Container":"webma,webm","Type":"Audio"},{"Container":"wav","Type":"Audio","AudioCodec":"PCM_S16LE,PCM_S24LE"},{"Container":"ogg","Type":"Audio"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis,opus","VideoCodec":"av1,VP8,VP9"}],"TranscodingProfiles":[],"ContainerProfiles":[],"CodecProfiles":[],"SubtitleProfiles":[{"Format":"vtt","Method":"Hls"},{"Format":"eia_608","Method":"VideoSideData","Protocol":"hls"},{"Format":"eia_708","Method":"VideoSideData","Protocol":"hls"},{"Format":"vtt","Method":"External"},{"Format":"ass","Method":"External"},{"Format":"ssa","Method":"External"}],"ResponseProfiles":[]}}
        );
        self.post(&path, &params, body).await?.json().await
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
            ("IncludeItemTypes", "Movie,Series,Video,Game,MusicAlbum"),
            ("SortBy", sortby),
            ("SortOrder", sort_order),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
            if listtype == "Genres" {
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

    pub async fn position_back(&self, back: &Back) -> Result<(), reqwest::Error> {
        let path = "Sessions/Playing/Progress".to_string();
        let params = [("reqformat", "json")];
        let body = json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":back.tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
        self.post(&path, &params, body).await?;
        Ok(())
    }

    pub async fn position_stop(&self, back: &Back) -> Result<(), reqwest::Error> {
        let path = "Sessions/Playing/Stopped".to_string();
        let params = [("reqformat", "json")];
        let body = json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":back.tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
        self.post(&path, &params, body).await?;
        Ok(())
    }

    pub async fn position_start(&self, back: &Back) -> Result<(), reqwest::Error> {
        let path = "Sessions/Playing".to_string();
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

    pub async fn get_favourite(&self, types: &str) -> Result<List, reqwest::Error> {
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
            ("Limit", "12"),
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
        self.post(&path, &[], body).await?.json().await
    }

    pub async fn hide_from_resume(&self, id: &str) -> Result<(), reqwest::Error> {
        let path = format!(
            "Users/{}/Items/{}/HideFromResume",
            &self.user_id.lock().unwrap(),
            id
        );
        let params = [("Hide", "true")];
        self.post(&path, &params, json!({})).await?.json().await
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

        let result = EMBY_CLIENT.search("你的名字", &["Movie"]);
        match result.await {
            Ok(items) => {
                for item in items.items {
                    println!("{}", item.name);
                }
                assert!(true);
            }
            Err(e) => {
                eprintln!("{}", e.to_user_facing());
                assert!(false);
            }
        }
    }
}
