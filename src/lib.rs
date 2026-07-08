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
mod playback;
mod steam;
mod subtitles;
mod tv;
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

use crate::{
    client::runtime::runtime,
    ui::widgets,
};

pub static USER_AGENT: LazyLock<String> =
    LazyLock::new(|| format!("{}/{} - {}", CLIENT_ID, version(), env::consts::OS));

pub const APP_ID: &str = "moe.tsuna.tsukimi";
pub const CLIENT_ID: &str = "Tsukimi";
const APP_RESOURCE_PATH: &str = "/moe/tsuna/tsukimi";
const GRESOURCE_FILE: &str = "tsukimi.gresource";

/// Runtime override for local dev (`just dev`, distrobox). Production builds use
/// compile-time `LOCALEDIR`; dev runs set `TSUKIMI_LOCALEDIR` because distrobox
/// compiles with `/run/host/...` paths that are invalid when launching on the host.
fn localizedir() -> &'static str {
    static RUNTIME: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        env::var("TSUKIMI_LOCALEDIR")
            .map(|path| Box::leak(path.into_boxed_str()) as &str)
            .unwrap_or(LOCALEDIR)
    })
}

fn pkgdatadir() -> std::path::PathBuf {
    env::var("TSUKIMI_PKGDATADIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(PKGDATADIR))
}

pub fn run() -> gtk::glib::ExitCode {
    let args = Args::parse();
    args.init();

    let tv_active = tv::resolve_tv_mode(args.tv_mode(), args.fullscreen());
    tv::set_tv_mode_active(tv_active);
    tracing::info!(
        "TV mode active: {tv_active} (cli={}, settings={})",
        args.tv_mode(),
        crate::ui::SETTINGS.tv_mode()
    );
    if tv_active && crate::steam::is_steam_big_picture() {
        tracing::info!("Steam Big Picture detected, enabling TV mode for this session");
    }

    // Initialize gettext
    setlocale(LocaleCategory::LcAll, String::new());
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").expect("Failed to set textdomain codeset");
    bindtextdomain(GETTEXT_PACKAGE, localizedir())
        .expect("Invalid argument passed to bindtextdomain");

    textdomain(GETTEXT_PACKAGE).expect("Invalid string passed to textdomain");

    adw::init().expect("Failed to initialize Adwaita");
    register_gio_resources();

    widgets::init();

    gtk::glib::set_application_name(CLIENT_ID);

    let _tokio_guard = runtime().enter();
    Application::new().run_with_args::<&str>(&[])
}

fn register_gio_resources() {
    let path = pkgdatadir().join(GRESOURCE_FILE);
    let resources = gtk::gio::Resource::load(path).expect("Failed to load resources.");
    gtk::gio::resources_register(&resources);
}
