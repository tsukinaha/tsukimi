use std::{
    ops::Deref,
    sync::OnceLock,
};

use dandanapi_client::*;
use mutsumi::Danmaku;

use crate::ui::mpv::danmaku::DanmakuExt;

const KEY: Option<&str> = option_env!("DANDANAPI_SECRET_KEY");
const CIPHERTEXT: &[u8] = include_bytes!("../../../secret/secret");
const X_APPID: &str = "e9imrhcexn";

#[derive(Clone)]
pub struct DanmakuClient(DanDanClient);

impl DanmakuClient {
    pub fn new() -> anyhow::Result<Self> {
        static INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
        let result = INIT.get_or_init(|| {
            let key = KEY
                .map(str::trim)
                .filter(|key| !key.is_empty())
                .ok_or_else(|| "DANDANAPI_SECRET_KEY is not configured".to_string())?;

            DanDanClient::init(
                X_APPID.to_string(),
                SecretGenerator::new(CIPHERTEXT.to_vec(), key.to_string()),
            )
            .map_err(|error| error.to_string())
        });

        if let Err(error) = result {
            anyhow::bail!(error.clone());
        }

        Ok(Self(DanDanClient::instance()))
    }
}

impl Deref for DanmakuClient {
    type Target = DanDanClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DanmakuClient {
    pub async fn search_episode(
        &self, params: SearchSearchEpisodesParams,
    ) -> anyhow::Result<Option<i64>> {
        let anime = self.route(SearchSearchEpisodes { params }).await?.animes;

        Ok(anime
            .unwrap_or_default()
            .into_iter()
            .flat_map(|anime| anime.episodes.unwrap_or_default())
            .find_map(|episode| episode.episode_id))
    }

    pub async fn get_comments(&self, episode_id: i64) -> anyhow::Result<Option<Vec<Danmaku>>> {
        let response = self
            .route(CommentGetComment {
                episode_id,
                params: CommentGetCommentParams {
                    from: 0,
                    with_related: true,
                    ch_convert: 0,
                },
            })
            .await?;

        let Some(comments) = response.comments else {
            return Ok(None);
        };

        let danmaku = comments
            .into_iter()
            .map(DanmakuExt::into_danmaku)
            .filter(|danmaku| !danmaku.content.is_empty())
            .collect::<Vec<_>>();

        Ok((!danmaku.is_empty()).then_some(danmaku))
    }
}
