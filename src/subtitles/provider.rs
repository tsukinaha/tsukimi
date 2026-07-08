use std::path::PathBuf;

use crate::subtitles::{
    opensubtitles::OpenSubtitlesProvider,
    subdl::SubDLProvider,
};

#[derive(Debug, Clone)]
pub struct SubtitleResult {
    pub provider: String,
    pub id: String,
    pub title: String,
    pub language: String,
    pub download_url: Option<String>,
}

#[async_trait::async_trait]
pub trait SubtitleProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn is_enabled(&self) -> bool;
    fn has_credentials(&self) -> bool {
        self.is_enabled()
    }
    async fn search(
        &self, query: &str, imdb_id: Option<&str>, language: &str,
    ) -> anyhow::Result<Vec<SubtitleResult>>;
    async fn download(&self, result: &SubtitleResult) -> anyhow::Result<PathBuf>;
}

pub struct SubtitleProviderRegistry {
    providers: Vec<Box<dyn SubtitleProvider>>,
}

impl Default for SubtitleProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SubtitleProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Box::new(OpenSubtitlesProvider::new()),
                Box::new(SubDLProvider::new()),
            ],
        }
    }

    pub fn searchable_providers(&self) -> Vec<&dyn SubtitleProvider> {
        self.providers
            .iter()
            .filter(|p| p.has_credentials())
            .map(|p| p.as_ref())
            .collect()
    }

    pub async fn search_all(
        &self, query: &str, imdb_id: Option<&str>, language: &str,
    ) -> Vec<SubtitleResult> {
        let providers = self.searchable_providers();
        if providers.is_empty() {
            tracing::warn!("no subtitle providers configured with API keys");
            return Vec::new();
        }
        let mut results = Vec::new();
        for provider in providers {
            if !provider.is_enabled() {
                tracing::info!(
                    "searching {} (API key configured; enable in Settings to use for auto-download)",
                    provider.display_name()
                );
            }
            match provider.search(query, imdb_id, language).await {
                Ok(mut found) => results.append(&mut found),
                Err(err) => tracing::warn!(
                    "subtitle search failed for {}: {err}",
                    provider.display_name()
                ),
            }
        }
        results
    }
}
