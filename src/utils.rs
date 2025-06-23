use std::{
    future::Future,
    path::PathBuf,
};

use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    client::{
        jellyfin_client::JELLYFIN_CLIENT,
        runtime::runtime,
    },
    ui::jellyfin_cache_path,
};

pub fn _spawn_tokio_blocking<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();

    runtime().spawn(async {
        let response = fut.await;
        sender.send(response)
    });
    receiver.blocking_recv().unwrap()
}

pub async fn spawn_tokio<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();

    runtime().spawn(async {
        let response = fut.await;
        sender.send(response)
    });
    receiver.await.unwrap()
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
    UseCacheIfAvailable,
    RefreshCache,
    #[allow(dead_code)]
    IgnoreCache,
    ReadCacheAndRefresh,
}

pub async fn fetch_with_cache<T, F>(
    cache_key: &str, cache_policy: CachePolicy, future: F,
) -> Result<T>
where
    T: for<'de> Deserialize<'de> + Serialize + Send + 'static,
    F: Future<Output = Result<T>> + Send + 'static,
{
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
    let update_in_background = matches!(cache_policy, CachePolicy::ReadCacheAndRefresh);

    if read_cache {
        if let Some(data) = read_from_cache(&path) {
            if update_in_background {
                let future = future;
                let path = path.to_owned();
                runtime().spawn(async move {
                    if let Ok(data) = future.await {
                        let _ = write_to_cache(&path, &data);
                    }
                });
            }
            return Ok(data);
        }
    }

    let data = spawn_tokio(future).await?;

    if write_cache {
        write_to_cache(&path, &data)?;
    }

    Ok(data)
}

fn read_from_cache<T>(path: &PathBuf) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
}

fn write_to_cache<T>(path: &PathBuf, data: &T) -> Result<()>
where
    T: Serialize,
{
    let serialized = serde_json::to_string(data)?;
    std::fs::write(path, serialized)?;
    Ok(())
}

pub async fn get_image_with_cache(id: String, img_type: String, tag: Option<u8>) -> Result<String> {
    runtime()
        .spawn(async move { JELLYFIN_CLIENT.get_image(&id, &img_type, tag).await })
        .await?
}
