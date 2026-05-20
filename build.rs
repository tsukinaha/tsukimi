#[cfg(windows)]
use tsukimi_buildscript::{
    gettext,
    potfiles,
    resources,
    windows,
};

fn main() {
    // These variables are passed by Meson, see src/config.rs.
    // We track them here to ensure Meson and direct Cargo calls can share the
    // same target directory safely.
    println!("cargo:rerun-if-env-changed=TSUKIMI_VERSION");
    println!("cargo:rerun-if-env-changed=TSUKIMI_LOCALEDIR");
    println!("cargo:rerun-if-env-changed=TSUKIMI_PKGDATADIR");

    #[cfg(windows)]
    {
        resources::compile();

        let translatable_files = potfiles::collect();
        potfiles::write(&translatable_files);
        gettext::update_pot(&translatable_files);
        gettext::compile_po();

        windows::embed_manifest();
    }
}
