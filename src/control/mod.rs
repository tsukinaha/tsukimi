// mod sidebar;
mod menu;
pub mod overlay;
//pub mod player;
pub mod scale;
pub mod toast;
pub mod volume_bar;

pub use menu::*;
pub use toast::*;
pub use volume_bar::*;

use gtk::prelude::*;

pub fn init() {
    MenuActions::ensure_type();
    VolumeBar::ensure_type();

    gtk::gio::resources_register_include!("mutsumi.gresource")
        .expect("Failed to register resources.");
}
