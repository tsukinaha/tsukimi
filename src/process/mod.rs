mod glib;
mod tokio;

pub use glib::{GlibTask, GlibTaskExt};
pub use tokio::{CacheConfig, Key, NoCache, TokioTask, TokioTaskExt};

pub trait Task {
    type Output;
    type Handle;

    fn spawn(self) -> Self::Handle;

    async fn run(self) -> Self::Output;
}
