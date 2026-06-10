mod menu;
pub mod overlay;
//pub mod player;
pub mod scale;
pub mod sidebar;
pub mod toast;
pub mod volume_bar;

pub use menu::*;
pub use scale::*;
pub use sidebar::*;
pub use toast::*;
pub use volume_bar::*;

use gtk::prelude::*;
use std::sync::Once;

pub fn register_resources() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        gtk::gio::resources_register_include!("mutsumi.gresource")
            .expect("Failed to register resources.");
    });
}

pub fn init() {
    register_resources();
    MenuActions::ensure_type();
    VideoScale::ensure_type();
    VolumeBar::ensure_type();
}
