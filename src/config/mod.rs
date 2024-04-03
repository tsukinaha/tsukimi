use std::{env, fs::File, io::Read};

use dirs::home_dir;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize)]
pub struct Config {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
}

pub fn load_cfg() {
    let mut path = home_dir().unwrap();
    path.push(".config");
    path.push("tsukimi.yaml");

    if path.exists() {
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let config: Config = serde_yaml::from_str(&contents).unwrap();
        env::set_var("EMBY_DOMAIN", &config.domain);
        env::set_var("EMBY_USERNAME", &config.username);
        env::set_var("EMBY_PASSWORD", &config.password);
        env::set_var("EMBY_PORT", &config.port);
        env::set_var("EMBY_USER_ID", &config.user_id);
        env::set_var("EMBY_ACCESS_TOKEN", &config.access_token);
    };
}

pub fn set_config() -> Config {
    let config = Config {
        domain: env::var("EMBY_DOMAIN").unwrap(),
        username: env::var("EMBY_USERNAME").unwrap(),
        password: env::var("EMBY_PASSWORD").unwrap(),
        port: env::var("EMBY_PORT").unwrap(),
        user_id: env::var("EMBY_USER_ID").unwrap(),
        access_token: env::var("EMBY_ACCESS_TOKEN").unwrap(),
    };
    config
}