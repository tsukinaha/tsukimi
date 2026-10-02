use std::{
    env,
    path::PathBuf,
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
use clap::Parser;
pub use config::*;
use gettextrs::*;
use gtk::prelude::*;

pub use ui::Window;

pub use app::TsukimiApplication as Application;

use crate::client::runtime::runtime;

pub static USER_AGENT: LazyLock<String> =
    LazyLock::new(|| format!("{}/{} - {}", CLIENT_ID, version(), env::consts::OS));

pub const APP_ID: &str = "moe.tsuna.tsukimi";
pub const CLIENT_ID: &str = "Tsukimi";
const APP_RESOURCE_PATH: &str = "/moe/tsuna/tsukimi";
const GRESOURCE_FILE: &str = "tsukimi.gresource";

pub fn run() -> gtk::glib::ExitCode {
    Args::parse().init();

    // Initialize gettext
    unsafe { setlocale(LocaleCategory::LcAll, String::new()) };
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").expect("Failed to set textdomain codeset");
    bindtextdomain(GETTEXT_PACKAGE, runtime_locale_dir())
        .expect("Invalid argument passed to bindtextdomain");

    textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");

    adw::init().expect("Failed to initialize Adwaita");
    mutsumi::init();

    register_gio_resources();

    ui::init();

    gtk::glib::set_application_name(CLIENT_ID);

    let _tokio_guard = runtime().enter();
    Application::new().run_with_args::<&str>(&[])
}

fn runtime_install_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().ok()?;
        return exe.parent()?.parent().map(std::path::Path::to_path_buf);
    }

    #[cfg(not(windows))]
    {
        None
    }
}

fn runtime_locale_dir() -> PathBuf {
    runtime_install_root()
        .map(|root| root.join("share").join("locale"))
        .unwrap_or_else(|| PathBuf::from(LOCALEDIR))
}

fn runtime_pkgdata_dir() -> PathBuf {
    runtime_install_root()
        .map(|root| root.join("share").join("tsukimi"))
        .unwrap_or_else(|| PathBuf::from(PKGDATADIR))
}

fn register_gio_resources() {
    let path = runtime_pkgdata_dir().join(GRESOURCE_FILE);
    let resources = gtk::gio::Resource::load(path).expect("Failed to load resources.");
    gtk::gio::resources_register(&resources);
}
