use std::{
    env,
    sync::LazyLock,
};

mod arg;
mod client;
mod config;
mod gstl;
mod macros;
mod ui;
mod utils;

#[cfg(target_os = "windows")]
pub use client::windows_compat::theme::is_system_dark_mode_enabled;

pub use arg::Args;
pub use config::{
    GETTEXT_PACKAGE,
    VERSION,
};
use once_cell::sync::OnceCell;
pub use ui::{
    build_ui,
    load_css,
};

pub static USER_AGENT: LazyLock<String> =
    LazyLock::new(|| format!("Tsukimi/{} - {}", VERSION, env::consts::OS));

pub const APP_ID: &str = "moe.tsuna.tsukimi";

#[cfg(target_os = "linux")]
const LINUX_LOCALEDIR: &str = "/usr/share/locale";
#[cfg(target_os = "windows")]
const WINDOWS_LOCALEDIR: &str = "share\\locale";

pub fn locale_dir() -> &'static str {
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
