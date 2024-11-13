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
        emby_client::EMBY_CLIENT,
        network::runtime,
    },
    ui::models::emby_cache_path,
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
    let mut path = emby_cache_path();
    path.push(format!("{}.json", cache_key));

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
                let path = path.clone();
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

pub async fn get_image_with_cache(id: &str, img_type: &str, tag: Option<u8>) -> Result<String> {
    let mut path = emby_cache_path();
    path.push(format!("{}-{}-{}", id, img_type, tag.unwrap_or(0)));

    let id = id.to_string();
    let img_type = img_type.to_string();

    if !path.exists() {
        let _ = runtime()
            .spawn(async move { EMBY_CLIENT.get_image(&id, &img_type, tag).await })
            .await;
    }

    Ok(path.to_string_lossy().to_string())
}
