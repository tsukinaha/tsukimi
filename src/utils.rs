use std::{
    future::Future,
    path::PathBuf,
};

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

pub fn spawn_tokio_without_await<F>(fut: F)
where
    F: std::future::Future + Send + 'static,
{
    runtime().spawn(async {
        let _ = fut.await;
    });
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
    #[allow(dead_code)]
    UseCacheIfAvailable,
    RefreshCache,
    #[allow(dead_code)]
    IgnoreCache,
    ReadCacheAndRefresh,
}

pub enum CacheSource {
    Cache,
    Network,
}

pub enum CacheEvent<T> {
    Data {
        #[allow(dead_code)]
        source: CacheSource,
        data: T,
    },
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

    let read_cache = matches!(
        cache_policy,
        CachePolicy::UseCacheIfAvailable | CachePolicy::ReadCacheAndRefresh
    );
    let write_cache = matches!(
        cache_policy,
        CachePolicy::UseCacheIfAvailable
            | CachePolicy::RefreshCache
            | CachePolicy::ReadCacheAndRefresh
    );

    let mut cache_hash = None;
    let cache_hit = read_cache
        && read_from_cache(&path).is_some_and(|cached| {
            cache_hash = Some(cached.hash);
            let _ = tx.try_send(CacheEvent::Data {
                source: CacheSource::Cache,
                data: cached.data,
            });
            true
        });

    let fetch_network = !cache_hit || matches!(cache_policy, CachePolicy::ReadCacheAndRefresh);
    if !fetch_network {
        return rx;
    }

    runtime().spawn(async move {
        match future.await {
            Ok(data) => {
                if write_cache {
                    match write_to_cache_if_changed(&path, &data, cache_hash) {
                        Ok(CacheWrite::Unchanged) => {
                            println!("Cache unchanged, not writing to disk");
                            return;
                        }
                        Ok(CacheWrite::Written) => {
                            println!("Cache updated");
                        }
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
