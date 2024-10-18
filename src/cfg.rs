use serde::{Deserialize, Serialize};

use crate::ui::provider::descriptor::VecSerialize;

pub const APP_VERSION: &str = "0.16.3";

#[derive(Serialize, Deserialize, Clone, PartialEq)]
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

impl VecSerialize<Account> for Vec<Account> {
    fn to_string(&self) -> String {
        serde_json::to_string(&self).expect("Failed to serialize Vec<Descriptor>")
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
