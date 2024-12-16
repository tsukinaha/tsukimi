use gtk::prelude::*;
use once_cell::sync::Lazy;
use reqwest::Client;

use crate::config::VERSION;

pub struct ReqClient;

static APP_USER_AGENT: Lazy<String> = Lazy::new(|| format!("Tsukimi/{}", VERSION));

impl ReqClient {
    pub fn build() -> Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);

        #[cfg(target_os = "linux")]
        let client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT.to_string())
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(settings.int("threads") as usize)
            .build()
            .expect("failed to initialize client");

        #[cfg(target_os = "windows")]
        let client = {
            let client_builder = reqwest::Client::builder()
                .user_agent(APP_USER_AGENT.to_string())
                .timeout(std::time::Duration::from_secs(10))
                .pool_max_idle_per_host(settings.int("threads") as usize);

            let client_builder = match get_proxy_settings() {
                Some(proxy_settings) => {
                    if let Ok(proxy) = reqwest::Proxy::all(proxy_settings) {
                        client_builder.proxy(proxy)
                    } else {
                        client_builder
                    }
                }
                _ => client_builder,
            };

            client_builder.build().expect("failed to initialize client")
        };

        client
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
        if WinHttpGetIEProxyConfigForCurrentUser(&mut proxy_config).is_err() {
            return None;
        }

        if !proxy_config.lpszProxy.is_null() {
            return PCWSTR(proxy_config.lpszProxy.0).to_string().ok();
        }

        if !proxy_config.lpszAutoConfigUrl.is_null() {
            return PCWSTR(proxy_config.lpszAutoConfigUrl.0)
                .to_string()
                .map(|proxy_url| {
                    proxy_url.split('/').collect::<Vec<_>>()[..3]
                        .join("/")
                        .to_string()
                })
                .ok();
        }
    }
    None
}

#[cfg(target_os = "linux")]
pub fn get_proxy_settings() -> Option<String> {
    use std::env;
    env::var("http_proxy")
        .or_else(|_| env::var("https_proxy"))
        .ok()
}
