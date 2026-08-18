mod format;
mod menu;
mod player;
mod scale;
mod scale_row;
mod sidebar;
mod toast;
mod volume_bar;

pub use format::*;
pub use menu::*;
pub use player::*;
pub use scale::*;
pub use scale_row::*;
pub use sidebar::*;
pub use toast::*;
pub use volume_bar::*;

use gtk::prelude::*;

// Call this after you have a display
pub fn init() {
    gtk::init().expect("Failed to initialize GTK.");

    gtk::gio::resources_register_include!("mutsumi.gresource")
        .expect("Failed to register resources.");

    MutsumiPlayer::ensure_type();
    ControlSidebar::ensure_type();
    MenuActions::ensure_type();
    VideoScale::ensure_type();
    ScaleRow::ensure_type();
    VolumeBar::ensure_type();

    let display = gtk::gdk::Display::default().expect("No default display");
    gtk::IconTheme::for_display(&display)
        .add_resource_path("/io/github/mutsumiuniverse/mutsumi/icons");

    let provider = gtk::CssProvider::new();
    provider.load_from_string(CONTROL_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

const CONTROL_CSS: &str = include_str!("style.css");
