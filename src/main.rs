#![cfg_attr(
    all(target_os = "windows", not(feature = "console")),
    windows_subsystem = "windows"
)]

use std::env;

use config::load_uuid;
use gtk::prelude::*;
use gtk::{gio, glib};
mod client;
mod config;
mod gstl;
mod macros;
mod ui;
mod utils;

const APP_ID: &str = "moe.tsuna.tsukimi";

fn main() -> glib::ExitCode {
    load_uuid();

    // redirect cache dir and set proxy env for gstreamer plugins
    #[cfg(windows)]
    {
        let settings = gtk::gio::Settings::new(APP_ID);

        // set proxy env
        env::set_var("http_proxy", settings.string("proxy"));
        env::set_var("https_proxy", settings.string("proxy"));

        // set cache dir
        env::set_var(
            "XDG_CACHE_HOME",
            env::current_exe()
                .unwrap()
                .ancestors()
                .nth(2)
                .unwrap()
                .join("cache"),
        );
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
