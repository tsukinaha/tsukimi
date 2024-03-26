use reqwest;
use serde::{Deserialize, Serialize};
use std::{env, fs};
use toml;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
    pub proxy: String,
    pub mpv: String,
}

pub struct ReqClient;

impl ReqClient {
    pub fn new() -> reqwest::Client {
        return reqwest::Client::new();
    }
    pub fn add_proxy() -> reqwest::Client {
        let proxy_setting = get_proxy_info();
        if proxy_setting.is_empty() {
            println!("no proxy set");
            return reqwest::Client::new();
        } else {
            let proxy = reqwest::Proxy::all(proxy_setting).expect("failed to find proxy");

            return reqwest::Client::builder()
                .proxy(proxy)
                .build()
                .expect("failed to initialize client");
        }
    }
}

pub struct MPVClient;

impl MPVClient {
    pub fn play(url: String, name: String) {
        let mut command = std::process::Command::new(mpv());
        let server_info = get_server_info();
        let titlename = format!("--force-media-title={}", name);
        let osdname = format!("--osd-playing-msg={}", name);
        let forcewindow = format!("--force-window=immediate");
        let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);
        if get_proxy_info().is_empty() {
            command
                .arg(forcewindow)
                .arg(titlename)
                .arg(osdname)
                .arg(url);
        } else {
            let proxy = format!("--http-proxy={}", get_proxy_info());
            command
                .arg(forcewindow)
                .arg(titlename)
                .arg(osdname)
                .arg(proxy)
                .arg(url);
        }
        command.spawn().expect("mpv failed to start");
    }

    pub fn play_with_sub(url: String, name: String, suburl: String) {
        let mut command = std::process::Command::new(mpv());
        let server_info = get_server_info();
        let titlename = format!("--force-media-title={}", name);
        let osdname = format!("--osd-playing-msg={}", name);
        let forcewindow = format!("--force-window=immediate");
        let sub = format!(
            "--sub-file={}:{}/emby{}",
            server_info.domain, server_info.port, suburl
        );
        let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);
        if get_proxy_info().is_empty() {
            command
                .arg(forcewindow)
                .arg(titlename)
                .arg(osdname)
                .arg(sub)
                .arg(url);
        } else {
            let proxy = format!("--http-proxy={}", get_proxy_info());
            command
                .arg(forcewindow)
                .arg(titlename)
                .arg(osdname)
                .arg(proxy)
                .arg(sub)
                .arg(url);
        }
        let _ = command.spawn().expect("mpv failed to start").wait();
    }
}

fn get_proxy_info() -> String {
    #[cfg(unix)]
    let config_path = dirs::home_dir().unwrap().join(".config/tsukimi.toml");

    #[cfg(windows)]
    let config_path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.toml");

    let data = std::fs::read_to_string(&config_path).unwrap();
    let config: Config = toml::from_str(&data).unwrap();
    return config.proxy;
}

fn default_mpv() -> String {
    #[cfg(unix)]
    return "mpv".to_string();

    #[cfg(windows)]
    return "mpv.exe".to_string();
}

fn mpv() -> String {
    #[cfg(unix)]
    let config_path = dirs::home_dir().unwrap().join(".config/tsukimi.toml");

    #[cfg(windows)]
    let config_path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.toml");

    let data = std::fs::read_to_string(&config_path).unwrap();
    let config: Config = toml::from_str(&data).unwrap();

    let mpv = config.mpv;
    if mpv.is_empty() {
        return default_mpv();
    } else {
        return mpv;
    }
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
    #[cfg(unix)]
    let mut path = dirs::home_dir()
        .unwrap()
        .join(".config")
        .join("tsukimi.toml");

    #[cfg(windows)]
    let path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.toml");

    if path.exists() {
        let data = fs::read_to_string(path).expect("read config file failed");
        let config: Config = toml::from_str(&data).expect("parse config data failed");
        server_info.domain = config.domain;
        server_info.user_id = config.user_id;
        server_info.access_token = config.access_token;
        server_info.port = config.port;
    };

    server_info
}
