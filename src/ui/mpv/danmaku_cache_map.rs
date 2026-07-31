use std::collections::HashMap;

use gtk::gio::prelude::SettingsExt;
use serde::{
    Deserialize,
    Serialize,
};

use crate::ui::{
    models::SETTINGS,
    provider::tu_item::TuItem,
};

const SETTINGS_KEY: &str = "danmaku-cache-map";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedDanmaku {
    pub episode_id: i64,
    pub item_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DanmakuCacheEntry {
    Series {
        season: u32,
        local_episode: u32,
        selected_episode: u32,
        episode_ids: Vec<i64>,
    },
    Item {
        episode_id: i64,
    },
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DanmakuCacheMap {
    entries: HashMap<String, DanmakuCacheEntry>,
}

impl DanmakuCacheMap {
    pub fn load() -> Self {
        serde_json::from_str(SETTINGS.string(SETTINGS_KEY).as_str()).unwrap_or_default()
    }

    pub fn cached_danmaku(&self, item: &TuItem) -> Option<CachedDanmaku> {
        let entry = self.entries.get(&Self::cache_key(item)?)?;
        let episode_id = match entry {
            DanmakuCacheEntry::Series {
                season,
                local_episode,
                selected_episode,
                episode_ids,
            } => {
                if item.parent_index_number() != *season || item.index_number() == 0 {
                    return None;
                }

                let index = i64::from(*selected_episode) + i64::from(item.index_number())
                    - i64::from(*local_episode);
                if index <= 0 {
                    return None;
                }
                *episode_ids.get(index as usize - 1)?
            }
            DanmakuCacheEntry::Item { episode_id } => *episode_id,
        };

        Some(CachedDanmaku {
            episode_id,
            item_name: Self::item_name(item),
        })
    }

    pub fn remember_manual_selection(
        &mut self, current_item: &TuItem, selected_episode: &TuItem, available_episodes: &[TuItem],
    ) -> anyhow::Result<()> {
        let key = Self::cache_key(current_item)
            .ok_or_else(|| anyhow::anyhow!("Current item has no stable cache key"))?;
        let selected_episode_id = Self::episode_id(selected_episode)?;

        let entry = if current_item.series_name().is_some() {
            let local_episode = current_item.index_number();
            let selected_episode = selected_episode.index_number();
            if local_episode == 0 || selected_episode == 0 {
                anyhow::bail!("Series episode index is unavailable");
            }

            let episode_ids = available_episodes
                .iter()
                .map(Self::episode_id)
                .collect::<anyhow::Result<Vec<_>>>()?;
            if episode_ids.is_empty() {
                anyhow::bail!("Danmaku episode list is empty");
            }

            DanmakuCacheEntry::Series {
                season: current_item.parent_index_number(),
                local_episode,
                selected_episode,
                episode_ids,
            }
        } else {
            DanmakuCacheEntry::Item {
                episode_id: selected_episode_id,
            }
        };

        self.entries.insert(key, entry);
        self.save()
    }

    fn save(&self) -> anyhow::Result<()> {
        let value = serde_json::to_string(self)?;
        SETTINGS.set_string(SETTINGS_KEY, &value)?;
        Ok(())
    }

    fn cache_key(item: &TuItem) -> Option<String> {
        if item.series_name().is_some() {
            if let Some(series_id) = item.series_id().filter(|id| !id.is_empty()) {
                return Some(format!("series-id:{series_id}"));
            }
            return item
                .series_name()
                .filter(|name| !name.trim().is_empty())
                .map(|name| format!("series-name:{}", name.trim().to_lowercase()));
        }

        let id = item.id();
        (!id.is_empty()).then(|| format!("item:{id}"))
    }

    fn episode_id(item: &TuItem) -> anyhow::Result<i64> {
        item.id()
            .parse()
            .map_err(|error| anyhow::anyhow!("Invalid danmaku episode ID: {error}"))
    }

    fn item_name(item: &TuItem) -> String {
        item.series_name().map_or_else(
            || item.name(),
            |series_name| format!("{} - {series_name}", item.name()),
        )
    }
}
