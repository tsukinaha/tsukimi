#![cfg_attr(
    all(target_os = "windows", not(feature = "console")),
    windows_subsystem = "windows"
)]

use clap::Parser;
use gettextrs::*;
use gtk::prelude::*;

use tsukimi::*;

fn main() -> gtk::glib::ExitCode {
    Args::parse().init();
    // Initialize gettext
    setlocale(LocaleCategory::LcAll, "");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").expect("Failed to set textdomain codeset");
    bindtextdomain(GETTEXT_PACKAGE, tsukimi::locale_dir())
        .expect("Invalid argument passed to bindtextdomain");

    textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");

    // Register and include resources
    gtk::gio::resources_register_include!("tsukimi.gresource")
        .expect("Failed to register resources.");

    // Initialize the GTK application
    gtk::glib::set_application_name("Tsukimi");

    // Create a new application
    let app = adw::Application::builder()
        .application_id(tsukimi::APP_ID)
        .resource_base_path("/moe/tsuna/tsukimi")
        .build();

    // Make Application detect Windows system dark mode
    #[cfg(target_os = "windows")]
    {
        use adw::prelude::AdwApplicationExt;

        use tsukimi::client::windows_compat::theme::is_system_dark_mode_enabled;

        if is_system_dark_mode_enabled() {
            app.style_manager()
                .set_color_scheme(adw::ColorScheme::PreferDark);
        }
    }

    // Load the CSS from the resource file
    app.connect_startup(|_| load_css());
    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    app.set_accels_for_action("win.about", &["<Ctrl>N"]);
    // Run the application
    app.run_with_args::<&str>(&[])
}
