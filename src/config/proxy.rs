use gtk::prelude::*;

pub struct ReqClient;

impl ReqClient {
    pub fn new() -> reqwest::Client {
        let settings = gtk::gio::Settings::new(crate::APP_ID);
        if !settings.string("proxy").is_empty() {
            let proxy = reqwest::Proxy::all(settings.string("proxy").to_string())
                .expect("failed to find proxy");
            return reqwest::Client::builder()
                .proxy(proxy)
                .build()
                .expect("failed to initialize client");
        } else {
            return reqwest::Client::new();
        }
    }
}
