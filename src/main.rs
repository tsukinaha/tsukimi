#![cfg_attr(
    all(target_os = "windows", not(feature = "console")),
    windows_subsystem = "windows"
)]

use arg::Args;
use clap::Parser;
use gettextrs::*;
use gtk::prelude::*;
use gtk::{gio, glib};
use once_cell::sync::OnceCell;
use std::env;
use tracing::info;

mod arg;
mod client;
mod config;
mod gstl;
mod macros;
mod ui;
mod utils;

const APP_ID: &str = "moe.tsuna.tsukimi";

const GETTEXT_PACKAGE: &str = "tsukimi";

#[cfg(target_os = "linux")]
const LINUX_LOCALEDIR: &str = "/usr/share/locale";
#[cfg(target_os = "windows")]
const WINDOWS_LOCALEDIR: &str = "share\\locale";

fn locale_dir() -> &'static str {
    static LOCALEDIR: OnceCell<&'static str> = OnceCell::new();
    LOCALEDIR.get_or_init(|| {
        #[cfg(target_os = "linux")]
        {
            LINUX_LOCALEDIR
        }
        #[cfg(target_os = "windows")]
        {
            let exe_path = std::env::current_exe().expect("Can not get locale dir");
            let locale_path = exe_path
                .ancestors()
                .nth(2)
                .expect("Can not get locale dir")
                .join(WINDOWS_LOCALEDIR);
            Box::leak(locale_path.into_boxed_path())
                .to_str()
                .expect("Can not get locale dir")
        }
    })
}

fn main() -> glib::ExitCode {
    let args = Args::parse();
    args.init_tracing_subscriber();
    args.init_gsk_renderer();

    // TODO: set this in args
    std::panic::set_hook(Box::new(|p| {
        tracing::error!("{p}");
    }));

    // Initialize gettext
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        setlocale(LocaleCategory::LcAll, "");
        bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
            .expect("Failed to set textdomain codeset");
        bindtextdomain(GETTEXT_PACKAGE, locale_dir())
            .expect("Invalid argument passed to bindtextdomain");

        textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");
    }

    #[cfg(target_os = "windows")]
    {
        // redirect cache dir to %LOCALAPPDATA%
        let config_local_dir = dirs::config_local_dir().expect("Failed to get %LOCALAPPDATA%");
        std::env::set_var("XDG_CACHE_HOME", config_local_dir);
    }

    info!(
        "Application Version: {}, Platform: {} {}, CPU Architecture: {}",
        config::APP_VERSION,
        env::consts::OS,
        env::consts::FAMILY,
        env::consts::ARCH
    );

    // Register and include resources
    gio::resources_register_include!("tsukimi.gresource").expect("Failed to register resources.");

    // Initialize the GTK application
    gtk::glib::set_application_name("Tsukimi");

    // Create a new application
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .resource_base_path("/moe/tsukimi")
        .build();

    // Make Application detect Windows system dark mode
    #[cfg(target_os = "windows")]
    {
        use crate::config::theme::is_system_dark_mode_enabled;
        use adw::prelude::AdwApplicationExt;

        if is_system_dark_mode_enabled() {
            app.style_manager()
                .set_color_scheme(adw::ColorScheme::PreferDark);
        }
    }

    // Load the CSS from the resource file
    app.connect_startup(|_| ui::load_css());
    // Connect to "activate" signal of `app`
    app.connect_activate(ui::build_ui);

    app.set_accels_for_action("win.about", &["<Ctrl>N"]);
    // Run the application
    app.run_with_args::<&str>(&[])
}
