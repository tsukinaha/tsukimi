use crate::ui::RUNTIME;

pub fn _spawn_tokio_blocking<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();

    RUNTIME.spawn(async {
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

    RUNTIME.spawn(async {
        let response = fut.await;
        sender.send(response)
    });
    receiver.await.unwrap()
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
