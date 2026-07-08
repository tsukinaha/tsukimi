use std::path::PathBuf;

use serde::Deserialize;

use super::{
    opensubtitles::subtitle_cache_path,
    provider::{
        SubtitleProvider,
        SubtitleResult,
    },
};
use crate::ui::SETTINGS;

const API_BASE: &str = "https://api.subdl.com/api/v1/subtitles";

pub struct SubDLProvider;

impl SubDLProvider {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct SubDLResponse {
    #[serde(default)]
    subtitles: Vec<SubDLEntry>,
}

#[derive(Deserialize)]
struct SubDLEntry {
    #[serde(default)]
    language: String,
    #[serde(default)]
    release_name: Option<String>,
    #[serde(default)]
    url: String,
}

#[async_trait::async_trait]
impl SubtitleProvider for SubDLProvider {
    fn id(&self) -> &'static str {
        "subdl"
    }

    fn display_name(&self) -> &'static str {
        "SubDL"
    }

    fn is_enabled(&self) -> bool {
        SETTINGS.subtitle_provider_enabled(self.id()) && !SETTINGS.subdl_api_key().is_empty()
    }

    fn has_credentials(&self) -> bool {
        !SETTINGS.subdl_api_key().trim().is_empty()
    }

    async fn search(
        &self, query: &str, imdb_id: Option<&str>, language: &str,
    ) -> anyhow::Result<Vec<SubtitleResult>> {
        let api_key = SETTINGS.subdl_api_key();
        if api_key.is_empty() {
            anyhow::bail!("SubDL API key is not configured");
        }

        let client = reqwest::Client::new();
        let mut request = client
            .get(API_BASE)
            .query(&[("api_key", api_key.as_str()), ("film_name", query)]);
        if !language.is_empty() {
            request = request.query(&[("languages", language.to_uppercase())]);
        }
        if let Some(imdb) = imdb_id.filter(|id| !id.is_empty()) {
            request = request.query(&[("imdb_id", imdb)]);
        }

        let response = request.send().await?;
        if !response.status().is_success() {
            anyhow::bail!("SubDL search failed: {}", response.status());
        }

        let payload: SubDLResponse = response.json().await?;
        Ok(payload
            .subtitles
            .into_iter()
            .enumerate()
            .filter(|(_, entry)| !entry.url.is_empty())
            .map(|(index, entry)| SubtitleResult {
                provider: self.id().to_string(),
                id: format!("subdl-{index}"),
                title: entry.release_name.unwrap_or_else(|| query.to_string()),
                language: entry.language,
                download_url: Some(entry.url),
            })
            .collect())
    }

    async fn download(&self, result: &SubtitleResult) -> anyhow::Result<PathBuf> {
        let url = result
            .download_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("SubDL result has no download URL"))?;
        let client = reqwest::Client::new();
        let sub_bytes = client.get(url).send().await?.bytes().await?;
        let path = subtitle_cache_path(&result.id, "srt");
        std::fs::write(&path, sub_bytes)?;
        Ok(path)
    }
}
