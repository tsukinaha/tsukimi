use once_cell::sync::Lazy;
pub mod settings;
pub use self::settings::Settings;
pub static SETTINGS: Lazy<Settings> = Lazy::new(Settings::default);
