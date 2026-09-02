pub const GETTEXT_PACKAGE: &str = "tsukimi";

pub const VERSION: &str = compile_version();

pub const LOCALEDIR: &str = match option_env!("TSUKIMI_LOCALEDIR") {
    Some(path) => path,
    None => "/usr/share/locale",
};

pub const PKGDATADIR: &str = match option_env!("TSUKIMI_PKGDATADIR") {
    Some(path) => path,
    None => "/usr/share/tsukimi",
};

pub const fn compile_version() -> &'static str {
    const fn fallback(version: Option<&'static str>) -> &'static str {
        match version {
            Some(version) => version,
            None => env!("CARGO_PKG_VERSION"),
        }
    }

    if option_env!("TSUKIMI_RELEASE").is_none() {
        return fallback(option_env!("TSUKIMI_GIT_VERSION"));
    };

    fallback(option_env!("TSUKIMI_VERSION"))
}

pub const fn version() -> &'static str {
    VERSION
}
