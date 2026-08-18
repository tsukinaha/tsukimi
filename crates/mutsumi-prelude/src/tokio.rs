use std::sync::OnceLock;

use tokio::runtime::{
    self,
    Runtime,
};

pub fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    })
}

pub fn throw<F>(fut: F)
where
    F: std::future::Future + Send + 'static,
{
    runtime().spawn(async {
        let _ = fut.await;
    });
}
