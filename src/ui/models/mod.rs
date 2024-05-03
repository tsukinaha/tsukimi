use once_cell::sync::Lazy;
pub mod settings;
pub use self::settings::Settings;
pub static SETTINGS: Lazy<Settings> = Lazy::new(Settings::default);

pub static CACHE_PATH: Lazy<std::path::PathBuf> = Lazy::new(|| {
    gtk::glib::user_cache_dir()
        .join("tsukimi")
});

pub fn emby_cache_path() -> std::path::PathBuf {
    CACHE_PATH
        .join(std::env::var("EMBY_NAME").unwrap())
}
