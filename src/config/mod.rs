use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{fs::File, io::Read};

pub mod proxy;

pub const APP_VERSION: &str = "0.14.0";

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

// Set %APPDATA%\tsukimi as config_dir on Windows
pub fn get_config_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let path = dirs::config_dir()
            .ok_or("Failed to get %APPDATA%")?
            .join("tsukimi");
        Ok(path)
    }

    #[cfg(unix)]
    {
        let path = dirs::config_dir().ok_or("Failed to get home directory")?;
        Ok(path)
    }
}

pub mod theme {
    #[cfg(target_os = "windows")]
    use windows::{core::*, Win32::System::Registry::*};

    /// Use windows crate to detect Windows system dark mode as gtk settings does not respect it
    #[cfg(target_os = "windows")]
    pub fn is_system_dark_mode_enabled() -> bool {
        #[cfg(windows)]
        unsafe {
            let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
            let mut key_handle = HKEY::default();

            let result = RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut key_handle);

            if result.is_err() {
                return false;
            }

            let mut data: u32 = 0;
            let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
            let value_name = w!("SystemUsesLightTheme");

            let result = RegQueryValueExW(
                key_handle,
                value_name,
                None,
                None,
                Some(&mut data as *mut u32 as *mut u8),
                Some(&mut data_size),
            );

            let _ = RegCloseKey(key_handle);

            if result.is_err() {
                return false;
            }

            data == 0
        }
    }
}
