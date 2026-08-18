use glib::*;

use crate::tokio::runtime;

pub trait UiAsyncExt {
    fn spawn_tokio<Fut, R>(self, fut: Fut, ui: impl FnOnce(R) + 'static + Send)
    where
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Send + 'static;
}

impl UiAsyncExt for MainContext {
    fn spawn_tokio<Fut, R>(self, fut: Fut, ui: impl FnOnce(R) + 'static + Send)
    where
        Fut: std::future::Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        runtime().spawn(async move {
            let r = fut.await;
            self.spawn_local(async move { ui(r) });
        });
    }
}

pub fn spawn_tokio_with_callback<Fut, R>(fut: Fut, ui: impl FnOnce(R) + 'static + Send)
where
    Fut: std::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    MainContext::default().spawn_tokio(fut, ui);
}

pub fn spawn_tokio_blocking<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    runtime().spawn_blocking(move || {
        f();
    });
}

pub fn spawn_local<Fut, R>(ui: impl FnOnce(R) + 'static + Send + Future)
where
    Fut: std::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    MainContext::default().spawn_local(ui);
}
