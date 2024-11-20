use gtk::prelude::*;
use once_cell::sync::Lazy;
use reqwest::Client;
use tower::limit::ConcurrencyLimit;

use crate::{
    config::VERSION,
    ui::models::SETTINGS,
};

pub struct ReqClient;

static APP_USER_AGENT: Lazy<String> = Lazy::new(|| format!("Tsukimi/{}", VERSION));

impl ReqClient {
    pub fn build() -> ConcurrencyLimit<Client> {
        let settings = gtk::gio::Settings::new(crate::APP_ID);

        let client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT.to_string())
            .timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(settings.int("threads") as usize)
            .build()
            .expect("failed to initialize client");

        tower::ServiceBuilder::new()
            .concurrency_limit(SETTINGS.threads() as usize)
            .service(client)
    }
}