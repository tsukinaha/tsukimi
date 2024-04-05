mod image;
mod moviedrop;
mod network;
mod new_dropsel;
mod provider;
mod widgets;
mod mpv;
use gtk::gdk::Display;
use gtk::{prelude::*, CssProvider};

pub fn build_ui(app: &adw::Application) {
    // Create new window and present it
    let window = widgets::window::Window::new(app);
    let about_action = gtk::gio::ActionEntry::builder("about")
            .activate(|_, _, _| {
                let about = adw::AboutWindow::builder()
                    .application_name("Tsukimi")
                    .version(crate::config::VERSION)
                    .comments("A simple third-party Emby client.\nTest version: tsukimi 0.3.0 \n2024.4.5 16:02")
                    .website("https://github.com/tsukinaha/tsukimi")
                    .application_icon("tsukimi")
                    .license_type(gtk::License::Gpl30)
                    .build();
                about.add_acknowledgement_section(Some("Code"),&["Inaha","Kosette"]);
                about.add_acknowledgement_section(Some("Special Thanks"), &["Qound","Eikano"]);
                about.present();
            })
            .build();
    window.add_action_entries([
            about_action,
    ]);
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
