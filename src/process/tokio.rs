use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use gtk::{
    glib,
    prelude::*,
};

use serde::{
    Deserialize,
    Serialize,
};

use ::tokio::task::JoinHandle;

use crate::client::runtime::runtime;
use crate::process::glib::GlibTask;
use crate::process::Task;
use crate::ui::jellyfin_cache_path;
use crate::utils::{
    CacheEvent,
    CachePolicy,
    CacheSource,
    CacheWrite,
    read_from_cache,
    write_to_cache_if_changed,
};

/// Implemented by parameter types that can produce a cache-file key.
///
/// Only required when calling [`TokioTask::enable_cache`]; tasks without
/// caching impose no constraint on their parameters.
pub trait Key {
    fn key(&self) -> String;
}

/// Type-state marker: task has no cache configured.
pub struct NoCache;

/// Type-state marker: task has caching enabled.
pub struct CacheConfig<T> {
    key: String,
    policy: CachePolicy,
    _phantom: PhantomData<T>,
}

// ── TokioTask ────────────────────────────────────────────────────────────────

pub struct TokioTask<W, D, Cache = NoCache>
where
    W: IsA<gtk::Widget> + 'static,
    D: Send + 'static,
{
    widget: glib::WeakRef<W>,
    glib_pre: Option<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    future: Pin<Box<dyn Future<Output = D> + Send + 'static>>,
    cache: Cache,
}

// ── Constructors & non-cached methods ────────────────────────────────────────

impl<W, D> TokioTask<W, D, NoCache>
where
    W: IsA<gtk::Widget> + 'static,
    D: Send + 'static,
{
    pub fn from_weak(
        widget: glib::WeakRef<W>,
        future: impl Future<Output = D> + Send + 'static,
    ) -> Self {
        TokioTask {
            widget,
            glib_pre: None,
            future: Box::pin(future),
            cache: NoCache,
        }
    }

    pub(crate) fn new_with_pre(
        widget: glib::WeakRef<W>,
        glib_pre: Pin<Box<dyn Future<Output = ()> + 'static>>,
        future: Pin<Box<dyn Future<Output = D> + Send + 'static>>,
    ) -> Self {
        TokioTask {
            widget,
            glib_pre: Some(glib_pre),
            future,
            cache: NoCache,
        }
    }

    pub fn then_glib<F>(self, callback: F) -> GlibTask<W, ()>
    where
        F: FnOnce(Option<W>, D) + 'static,
    {
        let (tx, rx) = flume::bounded::<D>(1);
        let widget = self.widget.clone();
        let future = self.future;
        let glib_pre = self.glib_pre;

        GlibTask::new(self.widget, async move {
            if let Some(pre) = glib_pre {
                glib::MainContext::default().spawn_local(pre);
            }
            runtime().spawn(async move {
                let result = future.await;
                let _ = tx.send_async(result).await;
            });
            if let Ok(result) = rx.recv_async().await {
                callback(widget.upgrade(), result);
            }
        })
    }
}

// ── enable_cache — only available when D = anyhow::Result<T> ─────────────────

impl<W, T> TokioTask<W, anyhow::Result<T>, NoCache>
where
    W: IsA<gtk::Widget> + 'static,
    T: Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    /// Enable file caching for this task.
    ///
    /// `params` supplies the cache key via [`Key::key`]; the `Key` bound is
    /// only imposed here — tasks that skip caching need no such constraint.
    pub fn enable_cache<P: Key>(
        self,
        params: &P,
        policy: CachePolicy,
    ) -> TokioTask<W, anyhow::Result<T>, CacheConfig<T>> {
        TokioTask {
            widget: self.widget,
            glib_pre: self.glib_pre,
            future: self.future,
            cache: CacheConfig {
                key: params.key(),
                policy,
                _phantom: PhantomData,
            },
        }
    }
}

// ── Cached then_glib ─────────────────────────────────────────────────────────

impl<W, T> TokioTask<W, anyhow::Result<T>, CacheConfig<T>>
where
    W: IsA<gtk::Widget> + 'static,
    T: Serialize + for<'de> Deserialize<'de> + Send + 'static,
{
    /// Attach a glib callback that is driven by [`CacheEvent`]s.
    ///
    /// Depending on the configured [`CachePolicy`] the callback may fire
    /// more than once (e.g. once for a cached result and once for the fresh
    /// network result), so it is `Fn` rather than `FnOnce`.
    pub fn then_glib<F>(self, callback: F) -> GlibTask<W, ()>
    where
        F: Fn(Option<W>, CacheEvent<T>) + 'static,
    {
        let widget = self.widget.clone();
        let future = self.future;
        let glib_pre = self.glib_pre;
        let CacheConfig { key, policy, .. } = self.cache;

        GlibTask::new(self.widget, async move {
            if let Some(pre) = glib_pre {
                glib::MainContext::default().spawn_local(pre);
            }

            let mut path = jellyfin_cache_path().await;
            path.push(format!("{key}.json"));

            let read_cache_data = matches!(
                policy,
                CachePolicy::UseCacheIfAvailable | CachePolicy::ReadCacheAndRefresh
            );
            let read_cache_hash = matches!(
                policy,
                CachePolicy::UseCacheIfAvailable
                    | CachePolicy::ReadCacheAndRefresh
                    | CachePolicy::RefreshAndEmitLatest
                    | CachePolicy::RefreshIfChanged
            );
            let write_cache = matches!(
                policy,
                CachePolicy::UseCacheIfAvailable
                    | CachePolicy::RefreshAndEmitLatest
                    | CachePolicy::RefreshIfChanged
                    | CachePolicy::ReadCacheAndRefresh
            );

            // Channel shared between the sync cache-read path and the async
            // network-fetch task.  Capacity 2 covers the cache-hit + network
            // result pair that ReadCacheAndRefresh can produce.
            let (tx, rx) = flume::bounded::<CacheEvent<T>>(2);

            let mut cache_hash = None;
            let mut cached_data = None;

            let cache_hit = read_cache_hash
                && read_from_cache::<T>(&path).is_some_and(|cached| {
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
                    policy,
                    CachePolicy::ReadCacheAndRefresh
                        | CachePolicy::RefreshAndEmitLatest
                        | CachePolicy::RefreshIfChanged
                );

            if fetch_network {
                let tx_net = tx.clone();
                runtime().spawn(async move {
                    match future.await {
                        Ok(data) => {
                            if write_cache {
                                match write_to_cache_if_changed(&path, &data, cache_hash) {
                                    Ok(CacheWrite::Unchanged) => {
                                        if let (
                                            CachePolicy::RefreshAndEmitLatest,
                                            Some(old),
                                        ) = (policy, cached_data)
                                        {
                                            let _ = tx_net
                                                .send_async(CacheEvent::Data {
                                                    source: CacheSource::Cache,
                                                    data: old,
                                                })
                                                .await;
                                        }
                                        return;
                                    }
                                    Ok(CacheWrite::Written) => {}
                                    Err(e) => {
                                        let _ = tx_net.send_async(CacheEvent::Error(e)).await;
                                        return;
                                    }
                                }
                            }
                            let _ = tx_net
                                .send_async(CacheEvent::Data {
                                    source: CacheSource::Network,
                                    data,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = tx_net.send_async(CacheEvent::Error(e)).await;
                        }
                    }
                });
            }

            // Drop the original sender so the channel closes as soon as the
            // network task (if any) drops its clone.
            drop(tx);

            while let Ok(event) = rx.recv_async().await {
                callback(widget.upgrade(), event);
            }
        })
    }
}

// ── Task impl (non-cached only) ───────────────────────────────────────────────

impl<W, D> Task for TokioTask<W, D, NoCache>
where
    W: IsA<gtk::Widget> + 'static,
    D: Send + 'static,
{
    type Output = D;
    type Handle = JoinHandle<D>;

    fn spawn(self) -> JoinHandle<D> {
        if let Some(pre) = self.glib_pre {
            glib::MainContext::default().spawn_local(pre);
        }
        runtime().spawn(self.future)
    }

    async fn run(self) -> D {
        if let Some(pre) = self.glib_pre {
            glib::MainContext::default().spawn_local(pre);
        }

        let (tx, rx) = flume::bounded::<D>(1);

        let future = self.future;
        runtime().spawn(async move {
            let result = future.await;
            let _ = tx.send_async(result).await;
        });

        rx.recv_async()
            .await
            .expect("tokio task dropped before sending result")
    }
}


pub trait TokioTaskExt: IsA<gtk::Widget> + Sized + 'static {
    fn tokio_task<D, F, Fut>(&self, f: F) -> TokioTask<Self, D>
    where
        D: Send + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = D> + Send + 'static;
}

impl<W: IsA<gtk::Widget> + Clone + 'static> TokioTaskExt for W {
    fn tokio_task<D, F, Fut>(&self, f: F) -> TokioTask<Self, D>
    where
        D: Send + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = D> + Send + 'static,
    {
        TokioTask::from_weak(self.downgrade(), f())
    }
}
