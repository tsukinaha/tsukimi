use reqwest::{
    Client,
    Proxy,
};

pub struct ReqClient;

impl ReqClient {
    pub fn build() -> Client {
        let mut client_builder = reqwest::Client::builder()
            .user_agent(crate::USER_AGENT.as_str())
            .timeout(std::time::Duration::from_secs(10));

        if let Some(proxy) = get_proxy_settings() {
            client_builder =
                client_builder.proxy(Proxy::all(proxy).expect("Failed to initialize proxy"));
        }

        client_builder.build().expect("Failed to initialize client")
    }
}

pub fn get_proxy_settings() -> Option<String> {
    use std::env;
    env::var("http_proxy")
        .or_else(|_| env::var("https_proxy"))
        .ok()
}
