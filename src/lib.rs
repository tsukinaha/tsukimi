use std::{
    env,
    sync::LazyLock,
};

mod app;
mod arg;
mod config;
mod gstl;
mod macros;
mod mpris_common;
mod ui;
mod utils;

pub mod client;

pub use arg::Args;
pub use config::GETTEXT_PACKAGE;
#[cfg(target_os = "linux")]
use config::LOCALEDIR;
use config::VERSION;
use once_cell::sync::OnceCell;

use clap::Parser;
use gettextrs::*;
use gtk::prelude::*;

pub use ui::Window;

pub use app::TsukimiApplication as Application;

pub static USER_AGENT: LazyLock<String> =
    LazyLock::new(|| format!("{}/{} - {}", CLIENT_ID, VERSION, env::consts::OS));

pub const APP_ID: &str = "moe.tsuna.tsukimi";
pub const CLIENT_ID: &str = "Tsukimi";
const APP_RESOURCE_PATH: &str = "/moe/tsuna/tsukimi";

#[cfg(target_os = "windows")]
const WINDOWS_LOCALEDIR: &str = "share\\locale";

pub fn locale_dir() -> &'static str {
    static FLOCALEDIR: OnceCell<&'static str> = OnceCell::new();
    FLOCALEDIR.get_or_init(|| {
        #[cfg(target_os = "linux")]
        {
            LOCALEDIR
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

pub fn run() -> gtk::glib::ExitCode {
    Args::parse().init();
    // Initialize gettext
    setlocale(LocaleCategory::LcAll, String::new());
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").expect("Failed to set textdomain codeset");
    bindtextdomain(GETTEXT_PACKAGE, locale_dir())
        .expect("Invalid argument passed to bindtextdomain");

    textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");

    adw::init().expect("Failed to initialize Adwaita");
    // Register and include resources
    gtk::gio::resources_register_include!("tsukimi.gresource")
        .expect("Failed to register resources.");

    danmakw::init();

    // Initialize the GTK application
    gtk::glib::set_application_name(CLIENT_ID);

    Application::new().run_with_args::<&str>(&[])
}
