#[cfg(unix)]
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Debug, Deserialize, Default)]
pub struct Config {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
    pub proxy: Option<String>,
    pub mpv: Option<String>,
}

fn get_proxy_info() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    let config_path = home_dir().unwrap().join(".config/tsukimi.yaml");

    #[cfg(windows)]
    let config_path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.yaml");

    let data = std::fs::read_to_string(&config_path).unwrap();
    let config: Config = serde_yaml::from_str(&data).unwrap();

    match config.proxy {
        Some(proxy) => Ok(proxy.clone()),
        None => Err("no proxy is found!".into()),
    }
}

fn client_with_proxy(proxy: String) -> reqwest::Client {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(proxy).expect("setting proxy failed"))
        .build()
        .expect("failed to init client");
    return client;
}

pub fn client() -> reqwest::Client {
    match get_proxy_info() {
        Ok(e) => client_with_proxy(e),
        Err(_) => reqwest::Client::new(),
    }
}

fn default_mpv() -> String {
    #[cfg(unix)]
    return "mpv".to_string();

    #[cfg(windows)]
    return "mpv.com".to_string();
}

pub fn mpv() -> String {
    #[cfg(unix)]
    let config_path = home_dir().unwrap().join(".config/tsukimi.yaml");

    #[cfg(windows)]
    let config_path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.yaml");

    let data = std::fs::read_to_string(&config_path).unwrap();
    let config: Config = serde_yaml::from_str(&data).unwrap();

    let mpv = match config.mpv {
        Some(mpv) => mpv,
        None => default_mpv(),
    };

    return mpv;
}
