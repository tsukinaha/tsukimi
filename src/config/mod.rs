use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{fs::File, io::Read};

pub mod proxy;

pub const APP_VERSION: &str = "0.12.2";

#[derive(Serialize, Debug, Deserialize)]
pub struct Config {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
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
    let mut path = dirs::config_dir().ok_or("Failed to get home directory")?;
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
    let mut path = dirs::config_dir().ok_or("Failed to get home directory")?;
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

pub fn remove(account: &Account) -> Result<(), Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir().ok_or("Failed to get home directory")?;
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
        .open(&path)?;
    writeln!(file, "{}", toml)?;
    Ok(())
}
