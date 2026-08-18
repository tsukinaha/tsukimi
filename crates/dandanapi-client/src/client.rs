use std::sync::{
    LazyLock,
    OnceLock,
};

use dandanapi::{
    ClientPrelude,
    Request,
    Route,
};

use crate::{
    DandanapiError,
    secret::{
        RequestHeaderGenerator,
        SecretGenerator,
    },
};

static HEADER_GENERATOR: OnceLock<RequestHeaderGenerator> = OnceLock::new();

#[derive(Clone, Debug, Default)]
pub struct DanDanClient {
    client: reqwest::Client,
}

impl DanDanClient {
    pub fn init(x_appid: String, secret_generator: SecretGenerator) -> crate::Result<()> {
        let request_header_generator = RequestHeaderGenerator::new(x_appid, secret_generator)?;
        HEADER_GENERATOR
            .set(request_header_generator)
            .map_err(|_| DandanapiError::SecretGenerationError("Already initialized".into()))
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub fn instance() -> Self {
        static INSTANCE: LazyLock<DanDanClient> = LazyLock::new(DanDanClient::default);
        INSTANCE.clone()
    }

    pub fn route<T>(&self, kind: T) -> Route<Self, T>
    where
        T: Request,
    {
        ClientPrelude::route(self, kind)
    }
}

impl ClientPrelude for DanDanClient {
    const BASE_URI: &'static str = "https://api.dandanplay.net";

    fn headers(&self, path: &str) -> Option<reqwest::header::HeaderMap> {
        HEADER_GENERATOR
            .get()
            .map(|generator| generator.header(path))
    }

    fn client(&self) -> reqwest::Client {
        self.client.clone()
    }
}
