use std::io;

use dandanapi::{
    CommentGetComment,
    CommentGetCommentParams,
    SearchSearchEpisodes,
    SearchSearchEpisodesParams,
};
use dandanapi_client::{
    DanDanClient,
    SecretGenerator,
};

const X_APPID: &str = "e9imrhcexn";
const EPISODE_TITLE: &str = "度过暑假的方法";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let key = include_str!("ed25519_key");
    let ciphertext = include_bytes!("secret");
    let generator = SecretGenerator::new(ciphertext.to_vec(), key.to_string());
    DanDanClient::init(X_APPID.to_string(), generator)?;
    let client = DanDanClient::instance();

    let response = client
        .route(SearchSearchEpisodes {
            params: SearchSearchEpisodesParams {
                anime: Some("夏日口袋".to_string()),
                tmdb_id: Some(271576),
                tmdb_id_type: 0,
                episode: None,
                v2: true,
            },
        })
        .await?;

    let episode_id = response
        .animes
        .as_deref()
        .unwrap_or_default()
        .iter()
        .flat_map(|anime| anime.episodes.as_deref().unwrap_or_default())
        .find(|episode| {
            episode
                .episode_title
                .as_deref()
                .is_some_and(|title| title.contains(EPISODE_TITLE))
        })
        .and_then(|episode| episode.episode_id)
        .ok_or_else(|| io::Error::other(format!("未找到剧集：{EPISODE_TITLE}")))?;

    let response = client
        .route(CommentGetComment {
            episode_id,
            params: CommentGetCommentParams {
                from: 0,
                with_related: true,
                ch_convert: 0,
            },
        })
        .await?;

    dbg!(response);

    Ok(())
}
