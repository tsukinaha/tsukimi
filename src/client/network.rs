use super::structs::*;
use crate::config::proxy::ReqClient;
use crate::config::{get_device_name, save_cfg, set_config, Account, APP_VERSION};
use crate::ui::models::SETTINGS;
use dirs::home_dir;
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Error};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::{env, fs, io::Write};
use tokio::runtime;

pub static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    runtime::Builder::new_multi_thread()
        .worker_threads(SETTINGS.threads() as usize)
        .enable_io()
        .enable_time()
        .build()
        .expect("Failed to create runtime")
});

pub fn client() -> &'static Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(ReqClient::build)
}

pub async fn loginv2(
    servername: String,
    server: String,
    username: String,
    password: String,
    port: String,
) -> Result<(), Error> {
    let mut headers = HeaderMap::new();
    headers.insert("X-Emby-Client", HeaderValue::from_static("Tsukimi"));
    headers.insert(
        "X-Emby-Device-Name",
        HeaderValue::from_str(&get_device_name()).unwrap(),
    );
    headers.insert(
        "X-Emby-Device-Id",
        HeaderValue::from_str(&env::var("UUID").unwrap()).unwrap(),
    );
    headers.insert(
        "X-Emby-Client-Version",
        HeaderValue::from_static(APP_VERSION),
    );
    headers.insert("X-Emby-Language", HeaderValue::from_static("zh-cn"));

    let body = json!({
        "Username": username,
        "Pw": password
    });

    let res = client()
        .post(&format!(
            "{}:{}/emby/Users/authenticatebyname",
            server, port
        ))
        .headers(headers)
        .json(&body)
        .send()
        .await?;
    let v: Value = res.json().await?;

    let user_id = v["User"]["Id"].as_str().unwrap();
    let access_token = v["AccessToken"].as_str().unwrap();

    let config = Account {
        servername,
        server,
        username,
        password,
        port,
        user_id: user_id.to_string(),
        access_token: access_token.to_string(),
    };
    save_cfg(config).await.unwrap();
    Ok(())
}

pub async fn search(searchinfo: String) -> Result<Vec<SearchResult>, Error> {
    let server = set_config();
    let mut model = SearchModel {
        search_results: Vec::new(),
    };
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server.domain, server.port, server.user_id
    );
    let params = [
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate",
        ),
        ("IncludeItemTypes", "Movie,Series"),
        ("StartIndex", "0"),
        ("SortBy", "SortName"),
        ("SortOrder", "Ascending"),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("ImageTypeLimit", "1"),
        ("Recursive", "true"),
        ("SearchTerm", &searchinfo),
        ("GroupProgramsBySeries", "true"),
        ("Limit", "50"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let mut json: serde_json::Value = response.json().await?;
    let items: Vec<SearchResult> = serde_json::from_value(json["Items"].take()).unwrap();
    model.search_results = items;
    Ok(model.search_results)
}

pub async fn get_series_info(id: String) -> Result<Vec<SeriesInfo>, Error> {
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Shows/{}/Episodes",
        server.domain, server.port, id
    );
    let params = [
        ("Fields", "Overview"),
        ("EnableTotalRecordCount", "true"),
        ("EnableImages", "true"),
        ("UserId", &server.user_id),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let mut json: serde_json::Value = response.json().await?;
    let seriesinfo: Vec<SeriesInfo> = serde_json::from_value(json["Items"].take()).unwrap();
    Ok(seriesinfo)
}

pub async fn get_item_overview(id: String) -> Result<Item, Error> {
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/{}",
        server.domain, server.port, server.user_id, id
    );
    let params = [
        ("Fields", "ShareLevel"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let item: Item = serde_json::from_value(json).unwrap();
    Ok(item)
}

pub async fn resume() -> Result<Vec<Resume>, Error> {
    let mut model = ResumeModel { resume: Vec::new() };
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/Resume",
        server.domain, server.port, server.user_id
    );
    let params = [
        ("Recursive", "true"),
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear",
        ),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("ImageTypeLimit", "1"),
        ("MediaTypes", "Video"),
        ("Limit", "8"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client().get(&url).query(&params).send().await?;
    let mut json: serde_json::Value = response.json().await?;
    let items: Vec<Resume> = serde_json::from_value(json["Items"].take()).unwrap();
    model.resume = items;
    Ok(model.resume)
}

pub async fn get_image(id: String, image_type: &str, tag: Option<u8>) -> Result<String, Error> {
    let server = set_config();

    let url = match image_type {
        "Pirmary" => format!(
            "{}:{}/emby/Items/{}/Images/Primary?maxHeight=400",
            server.domain, server.port, id
        ),
        "Backdrop" => format!(
            "{}:{}/emby/Items/{}/Images/Backdrop/{}?maxHeight=1200",
            server.domain,
            server.port,
            id,
            tag.unwrap()
        ),
        "Thumb" => format!(
            "{}:{}/emby/Items/{}/Images/Thumb?maxHeight=400",
            server.domain, server.port, id
        ),
        "Logo" => format!(
            "{}:{}/emby/Items/{}/Images/Logo?maxHeight=400",
            server.domain, server.port, id
        ),
        _ => format!(
            "{}:{}/emby/Items/{}/Images/Primary?maxHeight=400",
            server.domain, server.port, id
        ),
    };

    let path_str = format!(
        "{}/.local/share/tsukimi/{}",
        home_dir().expect("msg").display(),
        env::var("EMBY_NAME").unwrap()
    );
    let result = client().get(&url).send().await;

    match result {
        Ok(response) => {
            let bytes_result = response.bytes().await;
            match bytes_result {
                Ok(bytes) => {
                    if bytes.len() < 10240 {
                        return Ok(id);
                    }
                    let pathbuf = PathBuf::from(path_str);
                    if !pathbuf.exists() {
                        fs::create_dir_all(format!(
                            "{}/.local/share/tsukimi/{}",
                            home_dir().expect("msg").display(),
                            env::var("EMBY_NAME").unwrap()
                        ))
                        .unwrap();
                    }
                    match image_type {
                        "Primary" => {
                            fs::write(pathbuf.join(format!("{}.png", id)), &bytes).unwrap();
                        }
                        "Backdrop" => {
                            fs::write(
                                pathbuf.join(format!("b{}_{}.png", id, tag.unwrap())),
                                &bytes,
                            )
                            .unwrap();
                        }
                        "Thumb" => {
                            fs::write(pathbuf.join(format!("t{}.png", id)), &bytes).unwrap();
                        }
                        "Logo" => {
                            fs::write(pathbuf.join(format!("l{}.png", id)), &bytes).unwrap();
                        }
                        _ => {
                            fs::write(pathbuf.join(format!("{}.png", id)), &bytes).unwrap();
                        }
                    }
                    Ok(id)
                }
                Err(e) => {
                    eprintln!("loading error");
                    Err(e)
                }
            }
        }
        Err(e) => {
            eprintln!("loading error");
            Err(e)
        }
    }
}

pub async fn get_mediainfo(id: String) -> Result<Media, Error> {
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/{}",
        server.domain, server.port, server.user_id, id
    );
    let params = [
        ("Fields", "ShareLevel"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let mediainfo: Media = serde_json::from_value(json).unwrap();
    Ok(mediainfo)
}

pub async fn get_playbackinfo(id: String) -> Result<Media, Error> {
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Items/{}/PlaybackInfo",
        server.domain, server.port, id
    );

    let params = [
        ("StartTimeTicks", "0"),
        ("UserId", &server.user_id),
        ("AutoOpenLiveStream", "false"),
        ("IsPlayback", "false"),
        ("AudioStreamIndex", "1"),
        ("SubtitleStreamIndex", "1"),
        ("MaxStreamingBitrate", "160000000"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!(

        {"DeviceProfile":{"Name":"Direct play all","MaxStaticBitrate":1000000000,"MaxStreamingBitrate":1000000000,"MusicStreamingTranscodingBitrate":1500000,"DirectPlayProfiles":[{"Container":"mkv","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9,mp4","AudioCodec":"aac,ac3,alac,eac3,dts,flac,mp3,opus,truehd,vorbis"},{"Container":"mp4,m4v","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9","AudioCodec":"aac,alac,opus,mp3,flac,vorbis"},{"Container":"flv","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,mp3"},{"Container":"mov","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"opus","Type":"Audio"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3"},{"Container":"mp2,mp3","Type":"Audio","AudioCodec":"mp2"},{"Container":"m4a","AudioCodec":"aac","Type":"Audio"},{"Container":"mp4","AudioCodec":"aac","Type":"Audio"},{"Container":"flac","Type":"Audio"},{"Container":"webma,webm","Type":"Audio"},{"Container":"wav","Type":"Audio","AudioCodec":"PCM_S16LE,PCM_S24LE"},{"Container":"ogg","Type":"Audio"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis,opus","VideoCodec":"av1,VP8,VP9"}],"TranscodingProfiles":[],"ContainerProfiles":[],"CodecProfiles":[],"SubtitleProfiles":[{"Format":"vtt","Method":"Hls"},{"Format":"eia_608","Method":"VideoSideData","Protocol":"hls"},{"Format":"eia_708","Method":"VideoSideData","Protocol":"hls"},{"Format":"vtt","Method":"External"},{"Format":"ass","Method":"External"},{"Format":"ssa","Method":"External"}],"ResponseProfiles":[]}}

    );
    let response = client()
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let mediainfo: Media = serde_json::from_value(json).unwrap();
    Ok(mediainfo)
}

pub async fn get_sub(id: String, sourceid: String) -> Result<Media, Error> {
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Items/{}/PlaybackInfo",
        server.domain, server.port, id
    );

    let params = [
        ("StartTimeTicks", "0"),
        ("UserId", &server.user_id),
        ("AutoOpenLiveStream", "true"),
        ("IsPlayback", "true"),
        ("AudioStreamIndex", "1"),
        ("SubtitleStreamIndex", "1"),
        ("MediaSourceId", &sourceid),
        ("MaxStreamingBitrate", "4000000"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!(

        {"DeviceProfile":{"Name":"Direct play all","MaxStaticBitrate":1000000000,"MaxStreamingBitrate":1000000000,"MusicStreamingTranscodingBitrate":1500000,"DirectPlayProfiles":[{"Container":"mkv","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9,mp4","AudioCodec":"aac,ac3,alac,eac3,dts,flac,mp3,opus,truehd,vorbis"},{"Container":"mp4,m4v","Type":"Video","VideoCodec":"hevc,h264,av1,vp8,vp9","AudioCodec":"aac,alac,opus,mp3,flac,vorbis"},{"Container":"flv","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,mp3"},{"Container":"mov","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,opus,flac,vorbis"},{"Container":"opus","Type":"Audio"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3"},{"Container":"mp2,mp3","Type":"Audio","AudioCodec":"mp2"},{"Container":"m4a","AudioCodec":"aac","Type":"Audio"},{"Container":"mp4","AudioCodec":"aac","Type":"Audio"},{"Container":"flac","Type":"Audio"},{"Container":"webma,webm","Type":"Audio"},{"Container":"wav","Type":"Audio","AudioCodec":"PCM_S16LE,PCM_S24LE"},{"Container":"ogg","Type":"Audio"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis,opus","VideoCodec":"av1,VP8,VP9"}],"TranscodingProfiles":[],"ContainerProfiles":[],"CodecProfiles":[],"SubtitleProfiles":[{"Format":"vtt","Method":"Hls"},{"Format":"eia_608","Method":"VideoSideData","Protocol":"hls"},{"Format":"eia_708","Method":"VideoSideData","Protocol":"hls"},{"Format":"vtt","Method":"External"},{"Format":"ass","Method":"External"},{"Format":"ssa","Method":"External"}],"ResponseProfiles":[]}}

    );
    let response = client()
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let mediainfo: Media = serde_json::from_value(json).unwrap();
    Ok(mediainfo)
}

pub async fn get_library() -> Result<Vec<View>, Error> {
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Views",
        server.domain, server.port, server.user_id
    );

    let params = [
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let mut json: serde_json::Value = response.json().await?;
    let views: Vec<View> = serde_json::from_value(json["Items"].take()).unwrap();
    let views_json = serde_json::to_string(&views).unwrap();
    let mut pathbuf = PathBuf::from(format!(
        "{}/.local/share/tsukimi/{}",
        home_dir().expect("msg").display(),
        env::var("EMBY_NAME").unwrap()
    ));
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&pathbuf)
        .unwrap();
    pathbuf.push("views.json");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&pathbuf)
        .unwrap();
    writeln!(file, "{}", views_json).unwrap();
    Ok(views)
}

pub async fn get_latest(id: String) -> Result<Vec<Latest>, Error> {
    let server = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/Latest",
        server.domain, server.port, server.user_id
    );

    let params = [
        ("Limit", "16"),
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear",
        ),
        ("ParentId", &id),
        ("ImageTypeLimit", "1"),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let latests: Vec<Latest> = serde_json::from_value(json).unwrap();
    let latests_json = serde_json::to_string(&latests).unwrap();
    let mut pathbuf = PathBuf::from(format!(
        "{}/.local/share/tsukimi/{}",
        home_dir().expect("msg").display(),
        env::var("EMBY_NAME").unwrap(),
    ));
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&pathbuf)
        .unwrap();
    pathbuf.push(format!("latest_{}.json", id));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&pathbuf)
        .unwrap();
    writeln!(file, "{}", latests_json).unwrap();

    Ok(latests)
}

pub async fn get_list(
    id: String,
    start: String,
    mutex: std::sync::Arc<tokio::sync::Mutex<()>>,
) -> Result<List, Error> {
    let _ = mutex.lock().await;
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server_info.domain, server_info.port, server_info.user_id
    );

    let params = [
        ("Limit", "50"),
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate",
        ),
        ("ParentId", &id),
        ("ImageTypeLimit", "1"),
        ("StartIndex", &start),
        ("Recursive", "True"),
        ("IncludeItemTypes", "Movie,Series,MusicAlbum"),
        ("SortBy", "DateCreated,SortName"),
        ("SortOrder", "Descending"),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let latests: List = serde_json::from_value(json).unwrap();
    Ok(latests)
}

pub async fn like(id: &str) -> Result<(), Error> {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/FavoriteItems/{}",
        server_info.domain, server_info.port, server_info.user_id, id
    );

    let params = [
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    client().post(&url).query(&params).send().await?;
    Ok(())
}

pub async fn unlike(id: &str) -> Result<(), Error> {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/FavoriteItems/{}/Delete",
        server_info.domain, server_info.port, server_info.user_id, id
    );

    let params = [
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    client().post(&url).query(&params).send().await?;
    Ok(())
}

pub async fn played(id: &str) -> Result<(), Error> {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/PlayedItems/{}",
        server_info.domain, server_info.port, server_info.user_id, id
    );

    let params = [
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    client().post(&url).query(&params).send().await?;
    Ok(())
}

pub async fn unplayed(id: &str) -> Result<(), Error> {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/PlayedItems/{}/Delete",
        server_info.domain, server_info.port, server_info.user_id, id
    );

    let params = [
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    client().post(&url).query(&params).send().await?;
    Ok(())
}

pub async fn positionback(back: Back) {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Sessions/Playing/Progress",
        server_info.domain, server_info.port
    );

    let params = [
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":back.tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
    client()
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await
        .unwrap();
}

pub async fn positionstop(back: Back) {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Sessions/Playing/Stopped",
        server_info.domain, server_info.port
    );

    let params = [
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":back.tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
    client()
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await
        .unwrap();
}

pub async fn playstart(back: Back) {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Sessions/Playing",
        server_info.domain, server_info.port
    );

    let params = [
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"VolumeLevel":100,"IsMuted":false,"IsPaused":false,"RepeatMode":"RepeatNone","SubtitleOffset":0,"PlaybackRate":1,"MaxStreamingBitrate":4000000,"PositionTicks":back.tick,"PlaybackStartTimeTicks":0,"SubtitleStreamIndex":1,"AudioStreamIndex":1,"BufferedRanges":[],"PlayMethod":"DirectStream","PlaySessionId":back.playsessionid,"MediaSourceId":back.mediasourceid,"CanSeek":true,"ItemId":back.id,"PlaylistIndex":0,"PlaylistLength":23,"NextMediaType":"Video"});
    client()
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await
        .unwrap();
}

pub async fn similar(id: &str) -> Result<Vec<SearchResult>, Error> {
    let mut model = SearchModel {
        search_results: Vec::new(),
    };
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Items/{}/Similar",
        server_info.domain, server_info.port, id
    );
    let params = [
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate",
        ),
        ("UserId", &server_info.user_id),
        ("ImageTypeLimit", "1"),
        ("Limit", "12"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client().get(&url).query(&params).send().await?;
    let mut json: serde_json::Value = response.json().await?;
    let items: Vec<SearchResult> = serde_json::from_value(json["Items"].take()).unwrap();
    model.search_results = items;
    Ok(model.search_results)
}

pub async fn person_item(id: &str, types: &str) -> Result<Vec<Item>, Error> {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server_info.domain, server_info.port, server_info.user_id
    );
    let params = [
        ("Fields", "PrimaryImageAspectRatio,ProductionYear"),
        ("PersonIds", id),
        ("Recursive", "true"),
        ("CollapseBoxSetItems", "false"),
        ("SortBy", "SortName"),
        ("SortOrder", "Ascending"),
        ("IncludeItemTypes", types),
        ("ImageTypeLimit", "1"),
        ("Limit", "12"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client().get(&url).query(&params).send().await?;
    let mut json: serde_json::Value = response.json().await?;
    let items: Vec<Item> = serde_json::from_value(json["Items"].take()).unwrap();
    Ok(items)
}

pub async fn get_search_recommend() -> Result<List, Error> {
    let server_info = set_config();
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server_info.domain, server_info.port, server_info.user_id
    );

    let params = [
        ("Limit", "20"),
        ("EnableTotalRecordCount", "false"),
        ("ImageTypeLimit", "0"),
        ("Recursive", "true"),
        ("IncludeItemTypes", "Movie,Series"),
        ("SortBy", "IsFavoriteOrLiked,Random"),
        ("Recursive", "true"),
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let latests: List = serde_json::from_value(json).unwrap();
    Ok(latests)
}

pub async fn like_item(types: &str) -> Result<Vec<Item>, Error> {
    let server_info = set_config();
    let url = if types == "People" {
        format!("{}:{}/emby/Persons", server_info.domain, server_info.port)
    } else {
        format!(
            "{}:{}/emby/Users/{}/Items",
            server_info.domain, server_info.port, server_info.user_id
        )
    };
    let params = [
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear",
        ),
        ("Filters", "IsFavorite"),
        ("Recursive", "true"),
        ("CollapseBoxSetItems", "false"),
        ("SortBy", "SortName"),
        ("SortOrder", "Ascending"),
        ("IncludeItemTypes", types),
        ("Limit", "12"),
        if types == "People" {
            ("UserId", &server_info.user_id)
        } else {
            ("", "")
        },
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &get_device_name()),
        ("X-Emby-Device-Id", &env::var("UUID").unwrap()),
        ("X-Emby-Client-Version", APP_VERSION),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client().get(&url).query(&params).send().await?;
    let mut json: serde_json::Value = response.json().await?;
    let items: Vec<Item> = serde_json::from_value(json["Items"].take()).unwrap();
    Ok(items)
}
