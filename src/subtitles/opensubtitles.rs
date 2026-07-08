use std::path::PathBuf;

use serde::Deserialize;

use super::provider::{
    SubtitleProvider,
    SubtitleResult,
};
use crate::ui::SETTINGS;

const API_BASE: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = "Tsukimi v26.7.2";

pub struct OpenSubtitlesProvider;

impl OpenSubtitlesProvider {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<SearchEntry>,
}

#[derive(Deserialize)]
struct SearchEntry {
    #[allow(dead_code)]
    id: String,
    attributes: SearchAttributes,
}

#[derive(Deserialize)]
struct SearchAttributes {
    language: String,
    #[serde(default)]
    release: Option<String>,
    feature_details: FeatureDetails,
    files: Vec<SubtitleFile>,
}

#[derive(Deserialize)]
struct FeatureDetails {
    #[serde(default)]
    title: Option<String>,
}

#[derive(Deserialize)]
struct SubtitleFile {
    file_id: i64,
}

#[derive(serde::Serialize)]
struct DownloadRequest {
    file_id: i64,
}

#[derive(Deserialize)]
struct DownloadResponse {
    link: String,
}

#[async_trait::async_trait]
impl SubtitleProvider for OpenSubtitlesProvider {
    fn id(&self) -> &'static str {
        "opensubtitles"
    }

    fn display_name(&self) -> &'static str {
        "OpenSubtitles"
    }

    fn is_enabled(&self) -> bool {
        SETTINGS.subtitle_provider_enabled(self.id())
            && !SETTINGS.opensubtitles_api_key().is_empty()
    }

    fn has_credentials(&self) -> bool {
        !SETTINGS.opensubtitles_api_key().trim().is_empty()
    }

    async fn search(
        &self, query: &str, imdb_id: Option<&str>, language: &str,
    ) -> anyhow::Result<Vec<SubtitleResult>> {
        let api_key = SETTINGS.opensubtitles_api_key();
        if api_key.is_empty() {
            anyhow::bail!("OpenSubtitles API key is not configured");
        }

        let client = reqwest::Client::new();
        let mut request = client
            .get(format!("{API_BASE}/subtitles"))
            .header("Api-Key", api_key)
            .header("User-Agent", USER_AGENT)
            .query(&[("query", query)]);

        if !language.is_empty() {
            request = request.query(&[("languages", language)]);
        }
        if let Some(imdb) = imdb_id.filter(|id| !id.is_empty()) {
            request = request.query(&[("imdb_id", imdb)]);
        }

        let response = request.send().await?;
        if !response.status().is_success() {
            anyhow::bail!("OpenSubtitles search failed: {}", response.status());
        }

        let payload: SearchResponse = response.json().await?;
        let mut results = Vec::new();
        for entry in payload.data {
            let Some(file) = entry.attributes.files.first() else {
                continue;
            };
            let title = entry
                .attributes
                .release
                .or(entry.attributes.feature_details.title)
                .unwrap_or_else(|| query.to_string());
            results.push(SubtitleResult {
                provider: self.id().to_string(),
                id: file.file_id.to_string(),
                title,
                language: entry.attributes.language,
                download_url: None,
            });
        }
        Ok(results)
    }

    async fn download(&self, result: &SubtitleResult) -> anyhow::Result<PathBuf> {
        let api_key = SETTINGS.opensubtitles_api_key();
        let file_id: i64 = result.id.parse()?;
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{API_BASE}/download"))
            .header("Api-Key", api_key)
            .header("User-Agent", USER_AGENT)
            .json(&DownloadRequest { file_id })
            .send()
            .await?;
        if !response.status().is_success() {
            anyhow::bail!("OpenSubtitles download failed: {}", response.status());
        }
        let payload: DownloadResponse = response.json().await?;
        let sub_bytes = client.get(payload.link).send().await?.bytes().await?;
        let path = subtitle_cache_path(&result.id, "srt");
        std::fs::write(&path, sub_bytes)?;
        Ok(path)
    }
}

pub(crate) fn subtitle_cache_path(id: &str, extension: &str) -> PathBuf {
    let dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("tsukimi")
        .join("subtitles");
    std::fs::create_dir_all(&dir).ok();
    dir.join(format!("{id}.{extension}"))
}
