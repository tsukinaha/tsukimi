use gtk::prelude::*;
use once_cell::sync::Lazy;

use crate::config::VERSION;

pub struct ReqClient;

static APP_USER_AGENT: Lazy<String> = Lazy::new(|| format!("Tsukimi/{}", VERSION));

impl ReqClient {
    pub fn build() -> reqwest::Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);

        let client_builder = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT.to_string())
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(settings.int("threads") as usize);

        if let Some(proxy_url) = get_proxy_settings() {
            match reqwest::Proxy::all(proxy_url) {
                Ok(proxy) => client_builder.proxy(proxy),
                Err(_) => {
                    tracing::warn!("Failed to set proxy settings, using default client");
                    client_builder
                }
            }
        } else {
            client_builder
        }
        .build()
        .expect("failed to initialize client")
    }
}

#[cfg(target_os = "windows")]
use windows::{
    core::PCWSTR,
    Win32::Networking::WinHttp::{
        WinHttpGetIEProxyConfigForCurrentUser,
        WINHTTP_CURRENT_USER_IE_PROXY_CONFIG,
    },
};

#[cfg(target_os = "windows")]
pub fn get_proxy_settings() -> Option<String> {
    unsafe {
        let mut proxy_config = WINHTTP_CURRENT_USER_IE_PROXY_CONFIG::default();

        if WinHttpGetIEProxyConfigForCurrentUser(&mut proxy_config).is_ok()
            && !proxy_config.lpszProxy.is_null()
        {
            let proxy = PCWSTR(proxy_config.lpszProxy.0).to_string().unwrap();
            return Some(proxy);
        }
    }
    None
}

#[cfg(target_os = "linux")]
pub fn get_proxy_settings() -> Option<String> {
    use std::env;

    env::var("http_proxy").or_else(|_| env::var("https_proxy")).ok()
}
