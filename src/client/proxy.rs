use gtk::prelude::*;
use reqwest::Client;

pub struct ReqClient;

impl ReqClient {
    pub fn build() -> Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);

        reqwest::Client::builder()
            .user_agent(crate::USER_AGENT.as_str())
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(settings.int("threads") as usize)
            .build()
            .expect("Failed to initialize client")
    }
}

pub fn get_proxy_settings() -> Option<String> {
    use std::env;
    env::var("http_proxy")
        .or_else(|_| env::var("https_proxy"))
        .ok()
}
