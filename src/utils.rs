use std::path::PathBuf;

use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};
use xxhash_rust::xxh3::xxh3_64;

use crate::{
    client::{
        jellyfin_client::JELLYFIN_CLIENT,
        runtime::runtime,
    },
    ui::jellyfin_cache_path,
};

pub async fn spawn_tokio<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(fut).await.unwrap()
}

pub async fn spawn_tokio_blocking<F, R>(fut: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    runtime().spawn_blocking(fut).await.unwrap()
}

pub fn spawn_tokio_without_await<F>(fut: F)
where
    F: std::future::Future + Send + 'static,
{
    runtime().spawn(async {
        let _ = fut.await;
    });
}

pub fn spawn_tokio_blocking_without_await<F>(fut: F)
where
    F: FnOnce() + Send + 'static,
{
    runtime().spawn_blocking(fut);
}

pub fn spawn<F>(fut: F)
where
    F: std::future::Future + 'static,
{
    let ctx = gtk::glib::MainContext::default();
    ctx.spawn_local(async move {
        fut.await;
    });
}

pub fn spawn_g_timeout<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    gtk::glib::spawn_future_local(async move {
        // Give the GLib event loop a whole 250ms to animate the NavigtionPage
        gtk::glib::timeout_future(std::time::Duration::from_millis(250)).await;
        future.await;
    });
}

pub enum CachePolicy {
    // If cache exists: emit Cache and skip network. Otherwise: fetch network, write cache, emit
    // Network.
    #[allow(dead_code)]
    UseCacheIfAvailable,
    // Fetch network and emit exactly one latest result.
    // If cache exists and network is unchanged: emit Cache.
    // If cache misses or network changed: write cache and emit Network.
    RefreshAndEmitLatest,
    // Fetch network; only write cache and emit Network when cache misses or content changed.
    RefreshIfChanged,
    // Always skip cache read/write. Fetch network and emit Network.
    #[allow(dead_code)]
    IgnoreCache,
    // If cache exists: emit Cache, then fetch network; if changed, write cache and emit Network.
    // If cache misses: fetch network, write cache, emit Network.
    ReadCacheAndRefresh,
}

pub enum CacheSource {
    Cache,
    Network,
}

pub enum CacheEvent<T> {
    Data { source: CacheSource, data: T },
    Error(anyhow::Error),
}

type CacheHash = u64;

struct Cached<T> {
    data: T,
    hash: CacheHash,
}

enum CacheWrite {
    Written,
    Unchanged,
}

pub async fn fetch_with_cache<T, F>(
    cache_key: &str, cache_policy: CachePolicy, future: F,
) -> tokio::sync::mpsc::Receiver<CacheEvent<T>>
where
    T: for<'de> Deserialize<'de> + Serialize + Send + 'static,
    F: Future<Output = Result<T>> + Send + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::channel(2);
    let mut path = jellyfin_cache_path().await;
    path.push(format!("{cache_key}.json"));

    let read_cache_data = matches!(
        cache_policy,
        CachePolicy::UseCacheIfAvailable | CachePolicy::ReadCacheAndRefresh
    );
    let read_cache_hash = matches!(
        cache_policy,
        CachePolicy::UseCacheIfAvailable
            | CachePolicy::ReadCacheAndRefresh
            | CachePolicy::RefreshAndEmitLatest
            | CachePolicy::RefreshIfChanged
    );
    let write_cache = matches!(
        cache_policy,
        CachePolicy::UseCacheIfAvailable
            | CachePolicy::RefreshAndEmitLatest
            | CachePolicy::RefreshIfChanged
            | CachePolicy::ReadCacheAndRefresh
    );

    let mut cache_hash = None;
    let mut cached_data = None;
    let cache_hit = read_cache_hash
        && read_from_cache(&path).is_some_and(|cached| {
            cache_hash = Some(cached.hash);
            if read_cache_data {
                let _ = tx.try_send(CacheEvent::Data {
                    source: CacheSource::Cache,
                    data: cached.data,
                });
            } else {
                cached_data = Some(cached.data);
            }
            true
        });

    let fetch_network = !cache_hit
        || matches!(
            cache_policy,
            CachePolicy::ReadCacheAndRefresh
                | CachePolicy::RefreshAndEmitLatest
                | CachePolicy::RefreshIfChanged
        );
    if !fetch_network {
        return rx;
    }

    runtime().spawn(async move {
        match future.await {
            Ok(data) => {
                if write_cache {
                    match write_to_cache_if_changed(&path, &data, cache_hash) {
                        Ok(CacheWrite::Unchanged) => {
                            if let (CachePolicy::RefreshAndEmitLatest, Some(data)) =
                                (cache_policy, cached_data)
                            {
                                let _ = tx
                                    .send(CacheEvent::Data {
                                        source: CacheSource::Cache,
                                        data,
                                    })
                                    .await;
                            }
                            return;
                        }
                        Ok(CacheWrite::Written) => {}
                        Err(e) => {
                            let _ = tx.send(CacheEvent::Error(e)).await;
                            return;
                        }
                    }
                }
                let _ = tx
                    .send(CacheEvent::Data {
                        source: CacheSource::Network,
                        data,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx.send(CacheEvent::Error(e)).await;
            }
        }
    });

    rx
}

fn read_from_cache<T>(path: &PathBuf) -> Option<Cached<T>>
where
    T: for<'de> Deserialize<'de>,
{
    std::fs::read_to_string(path).ok().and_then(|contents| {
        Some(Cached {
            data: serde_json::from_str(&contents).ok()?,
            hash: xxh3_64(contents.as_bytes()),
        })
    })
}

fn write_to_cache_if_changed<T>(
    path: &PathBuf, data: &T, old_hash: Option<CacheHash>,
) -> Result<CacheWrite>
where
    T: Serialize,
{
    let serialized = serde_json::to_string(data)?;
    let new_hash = xxh3_64(serialized.as_bytes());

    if old_hash.is_some_and(|h| h == new_hash) {
        return Ok(CacheWrite::Unchanged);
    }

    std::fs::write(path, serialized)?;
    Ok(CacheWrite::Written)
}

pub async fn get_image_with_cache(id: String, img_type: String, tag: Option<u8>) -> Result<String> {
    runtime()
        .spawn(async move { JELLYFIN_CLIENT.get_image(&id, &img_type, tag).await })
        .await?
}
