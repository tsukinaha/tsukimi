use std::{
    env,
    sync::LazyLock,
};

mod app;
mod arg;
mod config;
mod gstl;
mod macros;
#[cfg(target_os = "linux")]
mod mpris_common;
mod ui;
mod utils;

pub mod client;

pub use arg::Args;
pub use config::GETTEXT_PACKAGE;
use config::{
    LOCALEDIR,
    PKGDATADIR,
    version,
};
use once_cell::sync::OnceCell;

use clap::Parser;
use gettextrs::*;
use gtk::prelude::*;

pub use ui::Window;

pub use app::TsukimiApplication as Application;

use crate::ui::widgets;

pub static USER_AGENT: LazyLock<String> =
    LazyLock::new(|| format!("{}/{} - {}", CLIENT_ID, version(), env::consts::OS));

pub const APP_ID: &str = "moe.tsuna.tsukimi";
pub const CLIENT_ID: &str = "Tsukimi";
const APP_RESOURCE_PATH: &str = "/moe/tsuna/tsukimi";
const GRESOURCE_FILE: &str = "tsukimi.gresource";

pub fn locale_dir() -> &'static str {
    static FLOCALEDIR: OnceCell<&'static str> = OnceCell::new();
    FLOCALEDIR.get_or_init(|| LOCALEDIR)
}

pub fn run() -> gtk::glib::ExitCode {
    init_portable_runtime_paths();
    Args::parse().init();
    // Initialize gettext
    setlocale(LocaleCategory::LcAll, String::new());
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").expect("Failed to set textdomain codeset");
    bindtextdomain(GETTEXT_PACKAGE, locale_dir())
        .expect("Invalid argument passed to bindtextdomain");

    textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");

    adw::init().expect("Failed to initialize Adwaita");
    register_gio_resources();

    widgets::init();

    // Initialize the GTK application
    gtk::glib::set_application_name(CLIENT_ID);

    Application::new().run_with_args::<&str>(&[])
}

fn init_portable_runtime_paths() {
    let Ok(exe_path) = env::current_exe() else {
        return;
    };
    let Some(exe_dir) = exe_path.parent() else {
        return;
    };

    let schemas_dir = exe_dir.join("share").join("glib-2.0").join("schemas");
    if schemas_dir.exists() && env::var_os("GSETTINGS_SCHEMA_DIR").is_none() {
        unsafe { env::set_var("GSETTINGS_SCHEMA_DIR", schemas_dir) };
    }

    let data_dir = exe_dir.join("share");
    if data_dir.exists() && env::var_os("XDG_DATA_DIRS").is_none() {
        unsafe { env::set_var("XDG_DATA_DIRS", data_dir) };
    }
}

fn register_gio_resources() {
    let path = resource_file_path();
    let resources = gtk::gio::Resource::load(&path).unwrap_or_else(|error| {
        panic!(
            "Failed to load resources from {}: {}",
            path.display(),
            error
        )
    });
    gtk::gio::resources_register(&resources);
}

fn resource_file_path() -> std::path::PathBuf {
    let system_path = std::path::Path::new(PKGDATADIR).join(GRESOURCE_FILE);
    if system_path.exists() {
        return system_path;
    }

    if let Ok(exe_path) = env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        let portable_path = exe_dir
            .join("share")
            .join(env!("CARGO_PKG_NAME"))
            .join(GRESOURCE_FILE);
        if portable_path.exists() {
            return portable_path;
        }
    }

    system_path
}
