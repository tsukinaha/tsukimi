use gtk::prelude::*;
use once_cell::sync::Lazy;

use super::APP_VERSION;

pub struct ReqClient;

static APP_USER_AGENT: Lazy<String> = Lazy::new(|| format!("Tsukimi/{}", APP_VERSION));

impl ReqClient {
    pub fn build() -> reqwest::Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);

        let client_builder = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT.to_string())
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(settings.int("threads") as usize);

        if let Some(proxy_url) = get_proxy_settings() {
            let proxy = reqwest::Proxy::all(proxy_url)
                .expect("failed to find proxy");
            client_builder.proxy(proxy)
        } else {
            client_builder
        }
        .build()
        .expect("failed to initialize client")
    }
}

#[cfg(target_os = "windows")]
pub fn get_proxy_settings() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let internet_settings = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Internet Settings").ok()?;
    let proxy_server: String = internet_settings.get_value("ProxyServer").ok()?;
    Some(proxy_server)
}

#[cfg(target_os = "linux")]
pub fn get_proxy_settings() -> Option<String> {
    use std::env;

    env::var("http_proxy").or_else(|_| env::var("https_proxy")).ok()
}
