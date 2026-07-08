use std::collections::HashMap;

use crate::{
    client::{
        jellyfin_client::JELLYFIN_CLIENT,
        structs::MediaSource,
    },
    utils::spawn_tokio,
};

#[derive(Clone)]
pub struct TrickplayManifest {
    pub tile_width: u32,
    pub tile_height: u32,
    pub interval_seconds: f64,
    pub url_template: String,
}

#[derive(Default)]
pub struct TrickplayCache {
    tiles: HashMap<u32, Vec<u8>>,
}

impl TrickplayCache {
    pub fn tile_index_for_time(&self, manifest: &TrickplayManifest, seconds: f64) -> u32 {
        (seconds / manifest.interval_seconds).floor() as u32
    }

    pub fn preview_url(&self, manifest: &TrickplayManifest, seconds: f64) -> String {
        let index = self.tile_index_for_time(manifest, seconds);
        manifest.url_template.replace("{index}", &index.to_string())
    }

    pub fn cached_tile(&self, manifest: &TrickplayManifest, seconds: f64) -> Option<&[u8]> {
        let index = self.tile_index_for_time(manifest, seconds);
        self.tiles.get(&index).map(|data| data.as_slice())
    }

    pub fn store_tile(&mut self, manifest: &TrickplayManifest, seconds: f64, data: Vec<u8>) {
        let index = self.tile_index_for_time(manifest, seconds);
        self.tiles.insert(index, data);
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
    }
}

pub fn manifest_from_media_source(source: &MediaSource) -> Option<TrickplayManifest> {
    let trickplay = source.trickplay.as_ref()?;
    let first = trickplay.first()?;
    let url_template = first.url_template.clone()?;
    if url_template.is_empty() {
        return None;
    }
    Some(TrickplayManifest {
        tile_width: first.width.unwrap_or(160),
        tile_height: first.height.unwrap_or(90),
        interval_seconds: first.interval.unwrap_or(10) as f64,
        url_template,
    })
}

pub async fn fetch_trickplay_tile(url_path: &str) -> anyhow::Result<Vec<u8>> {
    let url_path = url_path.to_owned();
    spawn_tokio(async move { JELLYFIN_CLIENT.fetch_bytes(&url_path).await })
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))
}
