pub const GETTEXT_PACKAGE: &str = "tsukimi";

pub const VERSION: &str = match option_env!("TSUKIMI_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

pub const LOCALEDIR: &str = match option_env!("TSUKIMI_LOCALEDIR") {
    Some(path) => path,
    None => "/usr/share/locale",
};

pub const PKGDATADIR: &str = match option_env!("TSUKIMI_PKGDATADIR") {
    Some(path) => path,
    None => "/usr/share/tsukimi",
};

pub fn version() -> &'static str {
    VERSION
}
