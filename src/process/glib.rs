use std::future::Future;
use std::pin::Pin;

use gtk::{
    glib,
    prelude::*,
};

use crate::process::Task;
use crate::process::tokio::TokioTask;

pub struct GlibTask<W, T>
where
    W: IsA<gtk::Widget> + 'static,
    T: 'static,
{
    widget: glib::WeakRef<W>,
    future: Pin<Box<dyn Future<Output = T> + 'static>>,
}

impl<W, T> GlibTask<W, T>
where
    W: IsA<gtk::Widget> + 'static,
    T: 'static,
{
    pub(crate) fn new(
        widget: glib::WeakRef<W>,
        future: impl Future<Output = T> + 'static,
    ) -> Self {
        GlibTask {
            widget,
            future: Box::pin(future),
        }
    }

    pub fn then_tokio<D, F, Fut>(self, f: F) -> TokioTask<W, D>
    where
        T: Send + 'static,
        D: Send + 'static,
        F: FnOnce(T) -> Fut + Send + 'static,
        Fut: Future<Output = D> + Send + 'static,
    {
        let (tx, rx) = flume::bounded::<T>(1);
        let orig = self.future;

        let glib_pre = Box::pin(async move {
            let t = orig.await;
            let _ = tx.send_async(t).await;
        });

        let tokio_future = Box::pin(async move {
            let t = rx
                .recv_async()
                .await
                .expect("glib pre-task dropped before sending params");
            f(t).await
        });

        TokioTask::new_with_pre(self.widget, glib_pre, tokio_future)
    }
}

impl<W, T> Task for GlibTask<W, T>
where
    W: IsA<gtk::Widget> + 'static,
    T: 'static,
{
    type Output = T;
    type Handle = glib::JoinHandle<T>;

    fn spawn(self) -> glib::JoinHandle<T> {
        glib::MainContext::default().spawn_local(self.future)
    }

    async fn run(self) -> T {
        self.future.await
    }
}

pub trait GlibTaskExt: IsA<gtk::Widget> + Sized + 'static {
    fn glib_task<T, F>(&self, f: F) -> GlibTask<Self, T>
    where
        T: 'static,
        F: FnOnce(&Self) -> T + 'static;
}

impl<W: IsA<gtk::Widget> + Clone + 'static> GlibTaskExt for W {
    fn glib_task<T, F>(&self, f: F) -> GlibTask<Self, T>
    where
        T: 'static,
        F: FnOnce(&W) -> T + 'static,
    {
        let strong = self.clone();
        GlibTask::new(self.downgrade(), async move { f(&strong) })
    }
}
