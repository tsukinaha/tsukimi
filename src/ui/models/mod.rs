use once_cell::sync::Lazy;
pub mod settings;
pub use self::settings::Settings;
use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    utils::spawn_tokio_blocking,
};
pub static SETTINGS: Lazy<Settings> = Lazy::new(Settings::default);

pub static CACHE_PATH: Lazy<std::path::PathBuf> =
    Lazy::new(|| gtk::glib::user_cache_dir().join("tsukimi"));

pub async fn jellyfin_cache_path() -> std::path::PathBuf {
    let path = CACHE_PATH.join(JELLYFIN_CLIENT.server_name_hash.lock().await.as_str());
    spawn_tokio_blocking(|| {
        if !path.exists() {
            std::fs::create_dir_all(&path).expect("Failed to create directory");
        }
        path
    })
    .await
}
