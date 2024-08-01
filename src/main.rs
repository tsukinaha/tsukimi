#![windows_subsystem = "windows"]
use config::load_uuid;
use gettextrs::*;
use gtk::prelude::*;
use gtk::{gio, glib};

mod client;
mod config;
mod gstl;
mod macros;
mod ui;
mod utils;

const APP_ID: &str = "moe.tsuna.tsukimi";
const GETTEXT_PACKAGE: &str = "tsukimi";
const LOCALEDIR: &str = "/usr/share/locale";

fn main() -> glib::ExitCode {
    load_uuid();

    #[cfg(target_os = "linux")]
    {
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR)
            .expect("Invalid argument passed to bindtextdomain");
        textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");
    }

    // Register and include resources
    gio::resources_register_include!("tsukimi.gresource").expect("Failed to register resources.");

    // Initialize the GTK application
    adw::init().expect("Failed to initialize Adw");

    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();

    // load the icon theme
    let theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
    theme.add_resource_path("/moe/tsukimi/icons");

    // Load the CSS from the resource file
    app.connect_startup(|_| ui::load_css());
    // Connect to "activate" signal of `app`
    app.connect_activate(ui::build_ui);

    app.set_accels_for_action("win.about", &["<Ctrl>N"]);

    // Run the application
    app.run()
}
