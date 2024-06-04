use crate::ui::models::SETTINGS;
use once_cell::sync::Lazy;
use tokio::runtime;

pub static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    runtime::Builder::new_multi_thread()
        .worker_threads(SETTINGS.threads() as usize)
        .enable_io()
        .enable_time()
        .build()
        .expect("Failed to create runtime")
});
