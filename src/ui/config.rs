use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::BufReader;

#[derive(Serialize, Debug, Deserialize, Default)]
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

pub fn set_proxy(proxy: String) {
    #[cfg(unix)]
    let config_path = dirs::home_dir().unwrap().join(".config/tsukimi.yaml");

    #[cfg(windows)]
    let config_path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.yaml");

    let file = File::open(&config_path).expect("failed to open config file");
    let data = BufReader::new(file);
    let mut config: Config = serde_yaml::from_reader(data).expect("failed to parse YAML");

    config.proxy = proxy;

    let new_config = serde_yaml::to_string(&config).unwrap();

    std::fs::write(config_path, new_config).expect("写入代理配置失败");
}

pub fn get_proxy_info() -> String {
    #[cfg(unix)]
    let config_path = dirs::home_dir().unwrap().join(".config/tsukimi.yaml");

    #[cfg(windows)]
    let config_path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.yaml");

    let data = std::fs::read_to_string(&config_path).unwrap();
    let config: Config = serde_yaml::from_str(&data).unwrap();
    return config.proxy;
}

fn client_with_proxy(proxy: String) -> reqwest::Client {
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(proxy).expect("setting proxy failed"))
        .build()
        .expect("failed to init client");
    return client;
}

pub fn client() -> reqwest::Client {
    if get_proxy_info().is_empty() {
        return reqwest::Client::new();
    } else {
        return client_with_proxy(get_proxy_info());
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
    let config_path = dirs::home_dir().unwrap().join(".config/tsukimi.yaml");

    #[cfg(windows)]
    let config_path = env::current_dir()
        .unwrap()
        .join("config")
        .join("tsukimi.yaml");

    let data = std::fs::read_to_string(&config_path).unwrap();
    let config: Config = serde_yaml::from_str(&data).unwrap();

    let mpv = config.mpv;
    if mpv.is_empty() {
        return default_mpv();
    } else {
        return mpv;
    }
}
