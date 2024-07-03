use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{env, fs::File, io::Read};
use uuid::Uuid;

pub mod proxy;
pub const APP_VERSION: &str = "0.6.6";

#[derive(Serialize, Debug, Deserialize)]
pub struct Config {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
}

fn generate_uuid() -> String {
    let uuid = Uuid::new_v4();
    uuid.to_string()
}

pub fn load_uuid() {
    let uuid = generate_uuid();
    env::set_var("UUID", uuid);
}

pub fn set_config() -> Config {
    Config {
        domain: env::var("EMBY_DOMAIN").unwrap(),
        username: env::var("EMBY_USERNAME").unwrap(),
        password: env::var("EMBY_PASSWORD").unwrap(),
        port: env::var("EMBY_PORT").unwrap(),
        user_id: env::var("EMBY_USER_ID").unwrap(),
        access_token: env::var("EMBY_ACCESS_TOKEN").unwrap(),
    }
}

pub fn get_device_name() -> String {
    if cfg!(target_os = "windows") {
        env::var("COMPUTERNAME").unwrap_or("Unknown Device".to_string())
    } else {
        let output = std::process::Command::new("uname")
            .output()
            .expect("failed to execute process");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
    pub servername: String,
    pub server: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct Accounts {
    pub accounts: Vec<Account>,
}

pub async fn save_cfg(account: Account) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = get_config_dir()?;
    std::fs::DirBuilder::new().recursive(true).create(&path)?;
    path.push("tsukimi.toml");
    let mut accounts: Accounts = load_cfgv2()?;
    accounts.accounts.push(account);
    let toml = toml::to_string(&accounts).unwrap_or_else(|err| {
        eprintln!("Error while serializing accounts: {:?}", err);
        std::process::exit(1);
    });
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;
    writeln!(file, "{}", toml)?;
    Ok(())
}

pub fn load_cfgv2() -> Result<Accounts, Box<dyn std::error::Error>> {
    let mut path = get_config_dir()?;
    path.push("tsukimi.toml");
    if !path.exists() {
        return Ok(Accounts {
            accounts: Vec::new(),
        });
    }
    let mut file = File::open(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let accounts: Accounts = toml::from_str(&contents)?;
    Ok(accounts)
}

pub fn load_env(account: &Account) {
    env::set_var("EMBY_NAME", &account.servername);
    env::set_var("EMBY_DOMAIN", &account.server);
    env::set_var("EMBY_USERNAME", &account.username);
    env::set_var("EMBY_PASSWORD", &account.password);
    env::set_var("EMBY_PORT", &account.port);
    env::set_var("EMBY_USER_ID", &account.user_id);
    env::set_var("EMBY_ACCESS_TOKEN", &account.access_token);

    let uuid = generate_uuid();
    env::set_var("UUID", uuid);
}

pub fn remove(account: &Account) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = get_config_dir()?;
    path.push("tsukimi.toml");
    let mut accounts: Accounts = load_cfgv2()?;
    accounts.accounts.retain(|x| {
        x.servername != account.servername
            || x.server != account.server
            || x.username != account.username
            || x.password != account.password
            || x.port != account.port
            || x.user_id != account.user_id
            || x.access_token != account.access_token
    });
    let toml = toml::to_string(&accounts).unwrap_or_else(|err| {
        eprintln!("Error while serializing accounts: {:?}", err);
        std::process::exit(1);
    });
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    writeln!(file, "{}", toml)?;
    Ok(())
}

/// get config directory, not tsukimi.toml path
pub fn get_config_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let path = env::current_exe()?
            .ancestors()
            .nth(2)
            .ok_or("Failed to get Tsukimi root dir!")?
            .join("config");
        Ok(path)
    }

    #[cfg(unix)]
    {
        let path = dirs::home_dir()
            .ok_or("Failed to get Home dir!")?
            .join(".config");
        Ok(path)
    }
}
