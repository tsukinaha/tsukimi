use gtk::prelude::*;
use reqwest::Client;

pub struct ReqClient;

impl ReqClient {
    pub fn build() -> Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);

        #[cfg(target_os = "linux")]
        let client = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT.as_str())
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(settings.int("threads") as usize)
            .build()
            .expect("failed to initialize client");

        #[cfg(target_os = "windows")]
        let client = {
            let client_builder = reqwest::Client::builder()
                .user_agent(crate::USER_AGENT.as_str())
                .timeout(std::time::Duration::from_secs(10))
                .pool_max_idle_per_host(settings.int("threads") as usize);

            let client_builder = match get_proxy_settings() {
                Some(proxy_settings) => {
                    tracing::info!("Windows: Using proxy {}", proxy_settings);
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
    // FIXME: proxy should be a dynamic constructor
    //
    // This is only a temporary solution to get the proxy settings on Windows.
    // ProxyFactory::get_proxies() is a blocking method, PAC may be a stream.
    const EXAMPLE_PROXY: &str = "http://example.com";

    // FIEXME: user:password@ is not supported
    //
    // libproxy will return "direct://", if no proxy is found.
    // protocol://[user:password@]proxyhost[:port], but reqwest cant parse [user:password@]
    use libproxy::ProxyFactory;
    ProxyFactory::new()?
        .get_proxies(EXAMPLE_PROXY)
        .ok()?
        .first()
        .filter(|&proxy| proxy != "direct://")
        .cloned()
}

#[cfg(target_os = "linux")]
pub fn get_proxy_settings() -> Option<String> {
    use std::env;
    env::var("http_proxy")
        .or_else(|_| env::var("https_proxy"))
        .ok()
}
