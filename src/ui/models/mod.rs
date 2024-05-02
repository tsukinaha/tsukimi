use once_cell::sync::Lazy;
pub mod settings;
pub use self::settings::Settings;
pub static SETTINGS: Lazy<Settings> = Lazy::new(Settings::default);
use std::path::{Path, PathBuf};
use once_cell::sync::OnceCell;
use dirs::home_dir;

pub struct CachePath {
    path: OnceCell<PathBuf>,
}

impl CachePath {
    const CACHE_DIR: &'static str = ".local/share/tsukimi";

    pub fn new() -> Self {
        Self {
            path: OnceCell::new(),
        }
    }
    
    pub fn get(&self) -> &Path {
        self.path.get_or_init(|| {
            let mut path = home_dir().expect("Failed to get home directory");
            path.push(Self::CACHE_DIR);
            path
        })
    }

    pub fn with_emby_name(&self) -> &Path {
        self.path.get_or_init(|| {
            let mut path = home_dir().expect("Failed to get home directory");
            path.push(Self::CACHE_DIR);
            path.push(std::env::var("EMBY_NAME").unwrap());
            path
        })
    }
}