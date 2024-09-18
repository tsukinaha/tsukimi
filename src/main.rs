#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
const GETTEXT_PACKAGE: &str = "tsukimi";
#[cfg(target_os = "linux")]
const LOCALEDIR: &str = "/usr/share/locale";

fn main() -> glib::ExitCode {
    #[cfg(target_os = "linux")]
    {
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR)
            .expect("Invalid argument passed to bindtextdomain");
        textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");
    }

    // redirect cache dir to %LOCALAPPDATA%
    #[cfg(target_os = "windows")]
    {
        let config_local_dir = dirs::config_local_dir().expect("Failed to get %LOCALAPPDATA%");
        std::env::set_var("XDG_CACHE_HOME", config_local_dir);
    }

    // Initialize the logger
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Register and include resources
    gio::resources_register_include!("tsukimi.gresource").expect("Failed to register resources.");

    // Initialize the GTK application
    adw::init().expect("Failed to initialize Adw");

    // Make Application detect Windows system dark mode
    #[cfg(target_os = "windows")]
    {
        use crate::config::theme::is_system_dark_mode_enabled;

        if is_system_dark_mode_enabled() {
            let style_manager = adw::StyleManager::default();
            style_manager.set_color_scheme(adw::ColorScheme::PreferDark);
        }
    }

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
