pub const VERSION: &str = "25.05-danmaku-test-alpha01";
pub const GETTEXT_PACKAGE: &str = "tsukimi";

// If you are using meson, this will be replaced with the correct path.
// Otherwise, you can set it to the correct path where the locale files are installed.
//
// This value is reserved for build.rs.
#[cfg(target_os = "linux")]
pub const LOCALEDIR: &str = "/usr/share/locale";
