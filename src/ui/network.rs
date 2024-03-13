use crate::ui::settings_page::Config;
use gtk::gdk_pixbuf;

use dirs::home_dir;
use gdk_pixbuf::Pixbuf;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Error;
use serde_json::json;
use serde_json::Value;
use serde_yaml::to_string;
use std::fs::File;
use std::fs::{self, write};
use std::io::{Read, Write};
use std::path::PathBuf;
use tokio::runtime::Runtime;

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime::new().expect("Setting up tokio runtime needs to succeed."))
}

pub async fn login(
    domain: String,
    username: String,
    password: String,
    port: String,
) -> Result<(), Error> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("X-Emby-Client", HeaderValue::from_static("Emby Web"));
    headers.insert(
        "X-Emby-Device-Name",
        HeaderValue::from_static("Google Chrome Linux"),
    );
    headers.insert(
        "X-Emby-Device-Id",
        HeaderValue::from_static("23956a32-6628-47d0-a57d-b47a1f57aa02"),
    );
    headers.insert(
        "X-Emby-Client-Version",
        HeaderValue::from_static("4.8.0.54"),
    );
    headers.insert("X-Emby-Language", HeaderValue::from_static("zh-cn"));

    let body = json!({
        "Username": username,
        "Pw": password
    });

    let res = client
        .post(&format!(
            "{}:{}/emby/Users/authenticatebyname",
            domain, port
        ))
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    let v: Value = res.json().await?;

    // 获取 "User" 对象中的 "Id" 字段
    let user_id = v["User"]["Id"].as_str().unwrap();
    println!("UserId: {}", user_id);

    // 获取 "AccessToken" 字段
    let access_token = v["AccessToken"].as_str().unwrap();
    println!("AccessToken: {}", access_token);

    let config = Config {
        domain,
        username,
        password,
        port,
        user_id: user_id.to_string(),
        access_token: access_token.to_string(),
    };
    let yaml = to_string(&config).unwrap();
    let mut path = home_dir().unwrap();
    path.push(".config");
    path.push("tsukimi.yaml");
    write(path, yaml).unwrap();

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchResult {
    pub Name: String,
    pub Type: String,
    pub Id: String,
}

struct SearchModel {
    search_results: Vec<SearchResult>,
}

pub struct ServerInfo {
    pub domain: String,
    pub user_id: String,
    pub access_token: String,
    pub port: String,
}

pub fn get_server_info() -> ServerInfo {
    let mut server_info = ServerInfo {
        domain: String::new(),
        user_id: String::new(),
        access_token: String::new(),
        port: String::new(),
    };
    let mut path = home_dir().unwrap();
    path.push(".config");
    path.push("tsukimi.yaml");

    if path.exists() {
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let config: Config = serde_yaml::from_str(&contents).unwrap();
        server_info.domain = config.domain;
        server_info.user_id = config.user_id;
        server_info.access_token = config.access_token;
        server_info.port = config.port;
    };

    server_info
}

pub(crate) async fn search(searchinfo: String) -> Result<Vec<SearchResult>, Error> {
    let mut model = SearchModel {
        search_results: Vec::new(),
    };
    let server_info = get_server_info();

    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server_info.domain, server_info.port, server_info.user_id
    );
    let params = [
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate",
        ),
        ("StartIndex", "0"),
        ("SortBy", "SortName"),
        ("SortOrder", "Ascending"),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        ("ImageTypeLimit", "1"),
        ("Recursive", "true"),
        ("SearchTerm", &searchinfo),
        ("GroupProgramsBySeries", "true"),
        ("Limit", "50"),
        ("X-Emby-Client", "Emby+Web"),
        ("X-Emby-Device-Name", "Tsukimi"),
        ("X-Emby-Device-Id", "3d1edad3-27ff-46ff-9ec2-00643b1571cd"),
        ("X-Emby-Client-Version", "4.8.0.54"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let items: Vec<SearchResult> = serde_json::from_value(json["Items"].clone()).unwrap();
    model.search_results = items;
    Ok(model.search_results)
}

#[derive(Deserialize, Debug, Clone)]
pub struct SeriesInfo {
    pub Name: String,
    pub Id: String,
    pub Overview: Option<String>,
    pub IndexNumber: u32,
    pub ParentIndexNumber: u32,
}

pub struct seriesimage {
    pub image: Option<Pixbuf>,
}

pub async fn get_series_info(id: String) -> Result<Vec<SeriesInfo>, Error> {
    let server_info = get_server_info();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Shows/{}/Episodes",
        server_info.domain, server_info.port, id
    );
    let params = [
        ("Fields", "Overview"),
        ("EnableTotalRecordCount", "true"),
        ("EnableImages", "true"),
        ("UserId", &server_info.user_id),
        ("X-Emby-Client", "Emby+Web"),
        ("X-Emby-Device-Name", "Tsukimi"),
        ("X-Emby-Device-Id", "3d1edad3-27ff-46ff-9ec2-00643b1571cd"),
        ("X-Emby-Client-Version", "4.8.0.54"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let seriesinfo: Vec<SeriesInfo> = serde_json::from_value(json["Items"].clone()).unwrap();
    Ok(seriesinfo)
}

pub async fn get_image(id: String) -> Result<String, Error> {
    let server_info = get_server_info();
    let mut attempts = 0;

    loop {
        attempts += 1;
        let result = reqwest::get(&format!(
            "{}:{}/emby/Items/{}/Images/Primary",
            server_info.domain, server_info.port, id
        ))
        .await;

        match result {
            Ok(response) => {
                let bytes_result = response.bytes().await;
                match bytes_result {
                    Ok(bytes) => {
                        let path_str = format!("{}/.local/share/tsukimi/", home_dir().expect("msg").display());
                        let pathbuf = PathBuf::from(path_str);
                        if pathbuf.exists() {
                            fs::write(pathbuf.join(format!("{}.png",id)), &bytes).unwrap();
                        } else {
                            fs::create_dir_all(format!("{}/.local/share/tsukimi/", home_dir().expect("msg").display()))
                                .unwrap();
                            
                            fs::write(pathbuf.join(format!("{}.png",id)),&bytes).unwrap();
                        }
                        return Ok(id);
                    }
                    Err(e) => {
                        eprintln!("加载错误");
                        if attempts >= 3 {
                            return Err(e.into());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("加载错误");
                if attempts >= 3 {
                    return Err(e.into());
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaStream {
    pub DisplayTitle: Option<String>,
    pub Type: String,
    pub DeliveryUrl: Option<String>,
    pub IsExternal: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaSource {
    pub Id: String,
    pub Name: String,
    pub DirectStreamUrl: Option<String>,
    pub MediaStreams: Vec<MediaStream>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Media {
    pub MediaSources: Vec<MediaSource>,
}

pub async fn playbackinfo(id: String) -> Result<Media, Error> {
    let server_info = get_server_info();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Items/{}/PlaybackInfo",
        server_info.domain, server_info.port, id
    );

    let params = [
        ("StartTimeTicks", "0"),
        ("UserId", &server_info.user_id),
        ("AutoOpenLiveStream", "true"),
        ("IsPlayback", "true"),
        ("MaxStreamingBitrate", "4000000"),
        ("X-Emby-Client", "Emby+Web"),
        ("X-Emby-Device-Name", "Tsukimi"),
        ("X-Emby-Device-Id", "3d1edad3-27ff-46ff-9ec2-00643b1571cd"),
        ("X-Emby-Client-Version", "4.8.0.54"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let profile = serde_json::json!({"DeviceProfile":{"MaxStaticBitrate":140000000,"MaxStreamingBitrate":140000000,"MusicStreamingTranscodingBitrate":192000,"DirectPlayProfiles":[{"Container":"mp4,m4v","Type":"Video","VideoCodec":"h264,av1,vp8,vp9","AudioCodec":"mp3,aac,opus,flac,vorbis"},{"Container":"mkv","Type":"Video","VideoCodec":"h264,av1,vp8,vp9","AudioCodec":"mp3,aac,opus,flac,vorbis"},{"Container":"flv","Type":"Video","VideoCodec":"h264","AudioCodec":"aac,mp3"},{"Container":"3gp","Type":"Video","VideoCodec":"","AudioCodec":"mp3,aac,opus,flac,vorbis"},{"Container":"mov","Type":"Video","VideoCodec":"h264","AudioCodec":"mp3,aac,opus,flac,vorbis"},{"Container":"opus","Type":"Audio"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3"},{"Container":"mp2,mp3","Type":"Audio","AudioCodec":"mp2"},{"Container":"aac","Type":"Audio","AudioCodec":"aac"},{"Container":"m4a","AudioCodec":"aac","Type":"Audio"},{"Container":"mp4","AudioCodec":"aac","Type":"Audio"},{"Container":"flac","Type":"Audio"},{"Container":"webma,webm","Type":"Audio"},{"Container":"wav","Type":"Audio","AudioCodec":"PCM_S16LE,PCM_S24LE"},{"Container":"ogg","Type":"Audio"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis,opus","VideoCodec":"av1,VP8,VP9"}],"TranscodingProfiles":[{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Streaming","Protocol":"hls","MaxAudioChannels":"2","MinSegments":"1","BreakOnNonKeyFrames":true},{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"opus","Type":"Audio","AudioCodec":"opus","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"wav","Type":"Audio","AudioCodec":"wav","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"opus","Type":"Audio","AudioCodec":"opus","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp3","Type":"Audio","AudioCodec":"mp3","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"aac","Type":"Audio","AudioCodec":"aac","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"wav","Type":"Audio","AudioCodec":"wav","Context":"Static","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mkv","Type":"Video","AudioCodec":"mp3,aac,opus,flac,vorbis","VideoCodec":"h264,av1,vp8,vp9","Context":"Static","MaxAudioChannels":"2","CopyTimestamps":true},{"Container":"m4s,ts","Type":"Video","AudioCodec":"mp3,aac","VideoCodec":"h264","Context":"Streaming","Protocol":"hls","MaxAudioChannels":"2","MinSegments":"1","BreakOnNonKeyFrames":true,"ManifestSubtitles":"vtt"},{"Container":"webm","Type":"Video","AudioCodec":"vorbis","VideoCodec":"vpx","Context":"Streaming","Protocol":"http","MaxAudioChannels":"2"},{"Container":"mp4","Type":"Video","AudioCodec":"mp3,aac,opus,flac,vorbis","VideoCodec":"h264","Context":"Static","Protocol":"http"}],"ContainerProfiles":[],"CodecProfiles":[{"Type":"VideoAudio","Codec":"aac","Conditions":[{"Condition":"Equals","Property":"IsSecondaryAudio","Value":"false","IsRequired":"false"}]},{"Type":"VideoAudio","Conditions":[{"Condition":"Equals","Property":"IsSecondaryAudio","Value":"false","IsRequired":"false"}]},{"Type":"Video","Codec":"h264","Conditions":[{"Condition":"EqualsAny","Property":"VideoProfile","Value":"high|main|baseline|constrained baseline|high 10","IsRequired":false},{"Condition":"LessThanEqual","Property":"VideoLevel","Value":"62","IsRequired":false}]},{"Type":"Video","Codec":"hevc","Conditions":[]}],"SubtitleProfiles":[{"Format":"vtt","Method":"Hls"},{"Format":"eia_608","Method":"VideoSideData","Protocol":"hls"},{"Format":"eia_708","Method":"VideoSideData","Protocol":"hls"},{"Format":"vtt","Method":"External"},{"Format":"ass","Method":"External"},{"Format":"ssa","Method":"External"}],"ResponseProfiles":[{"Type":"Video","Container":"m4v","MimeType":"video/mp4"}]}});
    let response = client
        .post(&url)
        .query(&params)
        .json(&profile)
        .send()
        .await?;
    let json: serde_json::Value = response.json().await?;
    let mediainfo: Media = serde_json::from_value(json.clone()).unwrap();
    return Ok(mediainfo);
}

pub fn mpv_play(url: String, name: String) {
    let mut command = std::process::Command::new("mpv");
    let server_info = get_server_info();
    let titlename = format!("--force-media-title={}", name);
    let osdname = format!("--osd-playing-msg={}", name);
    let forcewindow = format!("--force-window=immediate");
    let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);
    command
        .arg(forcewindow)
        .arg(titlename)
        .arg(osdname)
        .arg(url);
    command.spawn().expect("mpv failed to start");
}

pub fn mpv_play_withsub(url: String, suburl: String, name: String) {
    let mut command = std::process::Command::new("mpv");
    let server_info = get_server_info();
    let titlename = format!("--force-media-title={}", name);
    let osdname = format!("--osd-playing-msg={}", name);
    let forcewindow = format!("--force-window=immediate");
    let sub = format!(
        "--sub-file={}:{}/emby{}",
        server_info.domain, server_info.port, suburl
    );
    let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);
    command
        .arg(forcewindow)
        .arg(titlename)
        .arg(osdname)
        .arg(sub)
        .arg(url);
    let _ = command.spawn().expect("mpv failed to start").wait();
}

pub async fn get_item_overview(id: String) -> Result<String, Error> {
    let server_info = get_server_info();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/{}",
        server_info.domain, server_info.port, server_info.user_id, id
    );
    let params = [
        ("Fields", "ShareLevel"),
        ("X-Emby-Client", "Emby+Web"),
        ("X-Emby-Device-Name", "Tsukimi"),
        ("X-Emby-Device-Id", "3d1edad3-27ff-46ff-9ec2-00643b1571cd"),
        ("X-Emby-Client-Version", "4.8.0.54"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let overview: String = serde_json::from_value(json["Overview"].clone()).unwrap();
    Ok(overview)
}

pub async fn markwatched(id: String, sourceid: String) -> Result<(String), Error> {
    let server_info = get_server_info();
    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/PlayingItems/{}",
        server_info.domain, server_info.port, server_info.user_id, id
    );
    println!("{}", url);
    let params = [
        ("X-Emby-Client", "Emby+Web"),
        ("X-Emby-Device-Name", "Tsukimi"),
        ("X-Emby-Device-Id", "3d1edad3-27ff-46ff-9ec2-00643b114514"),
        ("X-Emby-Client-Version", "4.8.0.54"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
        ("reqformat", "json"),
    ];
    let inplay = json!({
        "UserId": &server_info.user_id,
        "Id": &id,
        "MediaSourceId": &sourceid,
    });
    let response = client
        .post(&url)
        .query(&params)
        .json(&inplay)
        .send()
        .await?;
    let text = response.text().await?;
    Ok(text)
}

#[derive(Deserialize, Debug, Clone)]
pub struct Resume {
    pub Name: String,
    pub Type: String,
    pub Id: String,
    pub SeriesId: Option<String>,
    pub IndexNumber: Option<u32>,
    pub ParentIndexNumber: Option<u32>,
    pub ParentThumbItemId: Option<String>,
    pub SeriesName: Option<String>,
}

struct ResumeModel {
    resume: Vec<Resume>,
}

pub(crate) async fn resume() -> Result<Vec<Resume>, Error> {
    let mut model = ResumeModel { resume: Vec::new() };
    let server_info = get_server_info();

    let client = reqwest::Client::new();
    let url = format!(
        "{}:{}/emby/Users/{}/Items/Resume",
        server_info.domain, server_info.port, server_info.user_id
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
        ("X-Emby-Client", "Emby+Web"),
        ("X-Emby-Device-Name", "Tsukimi"),
        ("X-Emby-Device-Id", "3d1edad3-27ff-46ff-9ec2-00643b1571cd"),
        ("X-Emby-Client-Version", "4.8.0.54"),
        ("X-Emby-Token", &server_info.access_token),
        ("X-Emby-Language", "zh-cn"),
    ];

    let response = client.get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let items: Vec<Resume> = serde_json::from_value(json["Items"].clone()).unwrap();
    model.resume = items;
    Ok(model.resume)
}

pub async fn get_thumbimage(id: String) -> Result<String, Error> {
    let server_info = get_server_info();
    let mut attempts = 0;

    loop {
        attempts += 1;
        let result = reqwest::get(&format!(
            "{}:{}/emby/Items/{}/Images/Thumb",
            server_info.domain, server_info.port, id
        ))
        .await;

        match result {
            Ok(response) => {
                let bytes_result = response.bytes().await;
                match bytes_result {
                    Ok(bytes) => {
                        let path_str = format!("{}/.local/share/tsukimi/", home_dir().expect("msg").display());
                        let pathbuf = PathBuf::from(path_str);
                        if pathbuf.exists() {
                            fs::write(pathbuf.join(format!("t{}.png",id)), &bytes).unwrap();
                        } else {
                            fs::create_dir_all(format!("{}/.local/share/tsukimi/", home_dir().expect("msg").display()))
                                .unwrap();
                            
                            fs::write(pathbuf.join(format!("t{}.png",id)),&bytes).unwrap();
                        }
                        return Ok(id);
                    },
                    Err(e) => {
                        if attempts >= 3 {
                            return Err(e.into());
                        }
                    }
                }
            }
            Err(e) => {
                if attempts >= 3 {
                    return Err(e.into());
                }
            }
        }
    }
}

pub async fn get_backdropimage(id: String) -> Result<String, Error> {
    let server_info = get_server_info();
    let mut attempts = 0;

    loop {
        attempts += 1;
        let result = reqwest::get(&format!(
            "{}:{}/emby/Items/{}/Images/Backdrop",
            server_info.domain, server_info.port, id
        ))
        .await;

        match result {
            Ok(response) => {
                let bytes_result = response.bytes().await;
                match bytes_result {
                    Ok(bytes) => {
                        let path_str = format!("{}/.local/share/tsukimi/", home_dir().expect("msg").display());
                        let pathbuf = PathBuf::from(path_str);
                        if pathbuf.exists() {
                            fs::write(pathbuf.join(format!("b{}.png",id)), &bytes).unwrap();
                        } else {
                            fs::create_dir_all(format!("{}/.local/share/tsukimi/", home_dir().expect("msg").display()))
                                .unwrap();
                            
                            fs::write(pathbuf.join(format!("b{}.png",id)),&bytes).unwrap();
                        }
                        return Ok(id);
                    },
                    Err(e) => {
                        eprintln!("加载错误");
                        if attempts >= 3 {
                            return Err(e.into());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("加载错误");
                if attempts >= 3 {
                    return Err(e.into());
                }
            }
        }
    }
}
