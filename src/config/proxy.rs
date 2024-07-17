use gtk::prelude::*;
use once_cell::sync::Lazy;

use super::APP_VERSION;

pub struct ReqClient;

static APP_USER_AGENT: Lazy<String> = Lazy::new(|| format!("Tsukimi/{}", APP_VERSION));

impl ReqClient {
    pub fn build() -> reqwest::Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);
        if !settings.string("proxy").is_empty() {
            let proxy = reqwest::Proxy::all(settings.string("proxy").to_string())
                .expect("failed to find proxy");
            reqwest::Client::builder()
                .proxy(proxy)
                .user_agent(APP_USER_AGENT.to_string())
                .timeout(std::time::Duration::from_secs(10))
                .pool_max_idle_per_host(settings.int("threads") as usize)
                .build()
                .expect("failed to initialize client")
        } else {
            reqwest::Client::builder()
                .user_agent(APP_USER_AGENT.to_string())
                .timeout(std::time::Duration::from_secs(10))
                .pool_max_idle_per_host(settings.int("threads") as usize)
                .build()
                .expect("failed to initialize client")
        }
    }
}
