use gtk::prelude::*;

pub struct ReqClient;

impl ReqClient {
    pub fn build() -> reqwest::Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);
        if !settings.string("proxy").is_empty() {
            let proxy = reqwest::Proxy::all(settings.string("proxy").to_string())
                .expect("failed to find proxy");
            reqwest::Client::builder()
                .proxy(proxy)
                .build()
                .expect("failed to initialize client")
        } else {
            reqwest::Client::new()
        }
    }
}
