#![windows_subsystem = "windows"]
use gtk::prelude::*;
use gtk::{gio, glib};
mod config;
mod ui;

const APP_ID: &str = "moe.tsuna.tsukimi";

fn main() -> glib::ExitCode {
    // Load the configuration file
    config::load_cfg();
    // Register and include resources
    gio::resources_register_include!("tsukimi.gresource").expect("Failed to register resources.");

    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();
    // Load the CSS from the resource file
    app.connect_startup(|_| ui::load_css());
    // Connect to "activate" signal of `app`
    app.connect_activate(ui::build_ui);

    app.set_accels_for_action("win.about", &["<Ctrl>N"]);

    // Run the application
    app.run()
}
