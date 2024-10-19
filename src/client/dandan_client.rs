use std::{
    io::Write,
    sync::Mutex,
};

use anyhow::{
    anyhow,
    Result,
};
use chrono::NaiveDateTime;
use once_cell::sync::Lazy;
use reqwest::{
    header::HeaderValue,
    Method,
    RequestBuilder,
    Response,
};
use serde::{
    Deserialize,
    Serialize,
};
use url::Url;

use super::ReqClient;

const DANDANAPI: &str = "https://api.dandanplay.net";
const DANDANCASAPI: &str = "https://cas.dandanplay.net";
const DANDANAPI_SEARCH_ANIME_PATH: &str = "/api/v2/search/anime";
const DANDANAPI_SEARCH_EPISODE_PATH: &str = "/api/v2/search/episode";
const DANDANAPI_COMMENT_PATH: &str = "/api/comment/";

pub static DANDAN_CLIENT: Lazy<DanDanClient> = Lazy::new(DanDanClient::default);
pub struct DanDanClient {
    client: reqwest::Client,
    headers: Mutex<reqwest::header::HeaderMap>,
}

impl DanDanClient {
    pub fn default() -> Self {
        let client = ReqClient::build();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip"));
        Self { client, headers: Mutex::new(headers) }
    }

    pub async fn request<T>(&self, url: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let request = self.prepare_request(Method::GET, url, params)?;
        let res = self.send_request(request).await?.error_for_status()?;

        let json = res.json().await?;
        Ok(json)
    }

    fn prepare_request(
        &self, method: Method, url: &str, params: &[(&str, &str)],
    ) -> Result<RequestBuilder> {
        let mut url = Url::parse(url)?;
        self.add_params_to_url(&mut url, params);
        let headers = self.get_headers()?;
        Ok(self.client.request(method, url).headers(headers))
    }

    async fn send_request(&self, request: RequestBuilder) -> Result<Response> {
        let res = request.send().await?;
        Ok(res)
    }

    pub fn add_params_to_url(&self, url: &mut Url, params: &[(&str, &str)]) {
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, value);
        }
        tracing::info!("Request URL: {}", url);
    }

    pub fn get_headers(&self) -> Result<reqwest::header::HeaderMap> {
        let headers =
            self.headers.lock().map_err(|_| anyhow!("Failed to acquire lock on headers"))?.clone();
        Ok(headers)
    }

    pub async fn search_anime(&self, keyword: &str) -> Result<SearchAnimeResponse> {
        let params = &[("keyword", keyword)];
        self.request(&format!("{}{}", DANDANAPI, DANDANAPI_SEARCH_ANIME_PATH), params).await
    }

    pub async fn search_episode(&self, anime_id: u64) -> Result<SearchEpisodesResponse> {
        let anime_id_str = anime_id.to_string();
        let params = &[("anime", anime_id_str.as_str())];
        self.request(&format!("{}{}", DANDANAPI, DANDANAPI_SEARCH_EPISODE_PATH), params).await
    }

    pub async fn get_comments(&self, episode_id: u64) -> Result<()> {
        let request = self.prepare_request(
            Method::GET,
            &format!("{}{}{}", DANDANCASAPI, DANDANAPI_COMMENT_PATH, episode_id),
            &[],
        )?;
        let res = self.send_request(request).await?.error_for_status()?;
        let bytes = res.bytes().await?;

        let cache_path = crate::ui::models::comments_path();
        let comment_path = cache_path.join(episode_id.to_string());

        let mut file = std::fs::File::create(comment_path)?;
        file.write_all(&bytes)?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchAnimeResponse {
    /// List of anime works
    #[serde(rename = "animes")]
    pub animes: Option<Vec<SearchAnimeDetails>>,
    /// Error code, 0 means no error, non-zero means an error occurred
    #[serde(rename = "errorCode")]
    pub error_code: i32,
    /// Whether the API call was successful
    #[serde(rename = "success")]
    pub success: bool,
    /// Detailed error message when an error occurs
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchAnimeDetails {
    /// Anime ID
    #[serde(rename = "animeId")]
    pub anime_id: u64,
    /// Anime title
    #[serde(rename = "animeTitle")]
    pub title: Option<String>,
    /// Anime type
    #[serde(rename = "type")]
    pub anime_type: String,
    /// Type description
    #[serde(rename = "typeDescription")]
    pub type_description: Option<String>,
    /// Poster image URL
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    /// Release date
    #[serde(rename = "startDate")]
    pub start_date: NaiveDateTime,
    /// Total number of episodes
    #[serde(rename = "episodeCount")]
    pub episode_count: u64,
    /// Overall rating of the anime (0-10)
    #[serde(rename = "rating")]
    pub rating: f64,
    /// Whether the current user has favorited this anime
    #[serde(rename = "isFavorited")]
    pub is_favorited: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEpisodesResponse {
    /// Whether there are more undisplayed search results. This value is true
    /// when there are too many search results.
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    /// List of search results (anime information)
    #[serde(rename = "animes")]
    pub animes: Option<Vec<SearchEpisodesAnime>>,
    /// Error code, 0 means no error, non-zero means an error occurred
    #[serde(rename = "errorCode")]
    pub error_code: i32,
    /// Whether the API call was successful
    #[serde(rename = "success")]
    pub success: bool,
    /// Detailed error message when an error occurs
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEpisodesAnime {
    /// Anime ID
    #[serde(rename = "animeId")]
    pub anime_id: u64,
    /// Anime title
    #[serde(rename = "animeTitle")]
    pub title: Option<String>,
    /// Anime type
    #[serde(rename = "type")]
    pub anime_type: String,
    /// Type description
    #[serde(rename = "typeDescription")]
    pub type_description: Option<String>,
    /// List of episodes for this anime
    #[serde(rename = "episodes")]
    pub episodes: Option<Vec<SearchEpisodeDetails>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchEpisodeDetails {
    /// Episode ID (bullet screen library number)
    #[serde(rename = "episodeId")]
    pub episode_id: u64,
    /// Episode title
    #[serde(rename = "episodeTitle")]
    pub title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_anime() {
        match DANDAN_CLIENT.search_anime("进击的巨人").await {
            Ok(res) => {
                println!("{:?}", res);
            }
            Err(e) => {
                panic!("Failed to search anime: {:?}", e);
            }
        }

        match DANDAN_CLIENT.search_anime("").await {
            Ok(res) => {
                println!("{:?}", res);
            }
            Err(e) => {
                panic!("Failed to search anime: {:?}", e);
            }
        }

        match DANDAN_CLIENT.search_anime("sadiauhiuhliubalikubcksauiyg").await {
            Ok(res) => {
                println!("{:?}", res);
            }
            Err(e) => {
                panic!("Failed to search anime: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_write_comments() {
        match DANDAN_CLIENT.get_comments(145340001).await {
            Ok(()) => {
                println!("Comments written successfully");
            }
            Err(e) => {
                panic!("Failed to search episode: {:?}", e);
            }
        }
    }
}
