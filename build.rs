fn main() {
    // These variables are passed by Meson, see src/config.rs.
    // We track them here to ensure Meson and direct Cargo calls can share the
    // same target directory safely.
    println!("cargo:rerun-if-env-changed=TSUKIMI_VERSION");
    println!("cargo:rerun-if-env-changed=TSUKIMI_LOCALEDIR");
    println!("cargo:rerun-if-env-changed=TSUKIMI_PKGDATADIR");
}
