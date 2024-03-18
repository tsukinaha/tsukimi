mod config;
mod episodes_page;
mod home_page;
mod image;
mod item_page;
mod movie_page;
mod network;
mod new_dropsel;
mod search_page;
// mod settings_page;
mod widgets;

use gtk::gdk::Display;
use gtk::{prelude::*, CssProvider};

pub fn build_ui(app: &adw::Application) {
    // Create new window and present it
    let window = widgets::window::Window::new(app);
    window.present();
}

pub fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
