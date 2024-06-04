use super::structs::*;
use crate::config::proxy::ReqClient;
use crate::config::{get_device_name, set_config, APP_VERSION};
use crate::ui::models::SETTINGS;
use once_cell::sync::Lazy;
use reqwest::{Client, Error};
use std::env;
use std::sync::OnceLock;
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

pub async fn get_list(
    id: String,
    start: String,
    include_item_types: &str,
    listtype: &str,
    sort_order: &str,
    sortby: &str,
) -> Result<List, Error> {
    let server_info = set_config();
    let device_name = get_device_name();
    let device_id = env::var("UUID").unwrap();
    let app_version = APP_VERSION;
    let emby_token = server_info.access_token;
    let url = match listtype {
        "item" => format!(
            "{}:{}/emby/Users/{}/Items",
            server_info.domain, server_info.port, server_info.user_id
        ),
        "resume" => format!(
            "{}:{}/emby/Users/{}/Items/Resume",
            server_info.domain, server_info.port, server_info.user_id
        ),
        "genres" => format!("{}:{}/emby/Genres", server_info.domain, server_info.port),
        _ => format!(
            "{}:{}/emby/Users/{}/Items",
            server_info.domain, server_info.port, server_info.user_id
        ),
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
                ("StartIndex", &start),
                ("Recursive", "true"),
                ("IncludeItemTypes", include_item_type),
                ("SortBy", sortby),
                ("SortOrder", sort_order),
                ("EnableImageTypes", "Primary,Backdrop,Thumb"),
                if listtype == "liked" {("Filters", "IsFavorite")} else {("", "")},
                ("X-Emby-Client", "Tsukimi"),
                ("X-Emby-Device-Name", &device_name),
                ("X-Emby-Device-Id", &device_id),
                ("X-Emby-Client-Version", app_version),
                ("X-Emby-Token", &emby_token),
                ("X-Emby-Language", "zh-cn"),
                ]
        }

        "resume" => vec![
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
            ("X-Emby-Client", "Tsukimi"),
            ("X-Emby-Device-Name", &device_name),
            ("X-Emby-Device-Id", &device_id),
            ("X-Emby-Client-Version", app_version),
            ("X-Emby-Token", &emby_token),
            ("X-Emby-Language", "zh-cn"),
        ],

        "genres" => vec![
            ("Fields", "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio"),
            ("IncludeItemTypes", include_item_type),
            ("StartIndex", &start),
            ("ImageTypeLimit", "1"),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
            ("Limit", "50"),
            ("userId", &server_info.user_id),
            ("Recursive", "true"),
            ("ParentId", &id),
            ("SortBy", sortby),
            ("SortOrder", sort_order),
            ("X-Emby-Client", "Tsukimi"),
            ("X-Emby-Device-Name", &device_name),
            ("X-Emby-Device-Id", &device_id),
            ("X-Emby-Client-Version", app_version),
            ("X-Emby-Token", &emby_token),
            ("X-Emby-Language", "zh-cn"),
        ],
        _ => vec![],
    };
    let response = client().get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let l: List = serde_json::from_value(json).unwrap();
    Ok(l)
}

pub async fn get_inlist(
    id: String,
    start: String,
    listtype: &str,
    parentid: &str,
    sort_order: &str,
    sortby: &str,
) -> Result<List, Error> {
    let server_info = set_config();
    let device_name = get_device_name();
    let device_id = env::var("UUID").unwrap();
    let app_version = APP_VERSION;
    let emby_token = server_info.access_token;
    let url = format!(
        "{}:{}/emby/Users/{}/Items",
        server_info.domain, server_info.port, server_info.user_id
    );

    let params = vec![
        ("Limit", "50"),
        (
            "Fields",
            "BasicSyncInfo,CanDelete,PrimaryImageAspectRatio,ProductionYear,Status,EndDate,CommunityRating",
        ),
        ("ParentId", &id),
        ("ImageTypeLimit", "1"),
        ("StartIndex", &start),
        ("Recursive", "true"),
        ("IncludeItemTypes", "Movie,Series,Video,Game,MusicAlbum"),
        ("SortBy", sortby),
        ("SortOrder", sort_order),
        ("EnableImageTypes", "Primary,Backdrop,Thumb"),
        if listtype == "Genre" {
            ("GenreIds", parentid)
        } else {
            ("TagIds", parentid)
        },
        ("X-Emby-Client", "Tsukimi"),
        ("X-Emby-Device-Name", &device_name),
        ("X-Emby-Device-Id", &device_id),
        ("X-Emby-Client-Version", app_version),
        ("X-Emby-Token", &emby_token),
        ("X-Emby-Language", "zh-cn"),
    ];
    let response = client().get(&url).query(&params).send().await?;
    let json: serde_json::Value = response.json().await?;
    let l: List = serde_json::from_value(json).unwrap();
    Ok(l)
}
