use once_cell::sync::Lazy;
use tokio::runtime;

use crate::ui::models::SETTINGS;

pub static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    const STACK_SIZE: usize = 6 * 1024 * 1024;
    runtime::Builder::new_multi_thread()
        .worker_threads(SETTINGS.threads() as usize)
        .thread_stack_size(STACK_SIZE)
        .enable_all()
        .build()
        .expect("Failed to create runtime")
});
