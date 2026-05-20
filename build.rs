#[path = "buildscript/resources.rs"]
mod resources;

#[path = "buildscript/potfiles.rs"]
mod potfiles;

#[path = "buildscript/gschema.rs"]
mod gschema;

#[path = "buildscript/desktop.rs"]
mod desktop;

#[path = "buildscript/metainfo.rs"]
mod metainfo;

#[path = "buildscript/icons.rs"]
mod icons;

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[path = "buildscript/gettext.rs"]
mod gettext;

#[cfg(windows)]
#[path = "buildscript/windows.rs"]
mod windows;

fn main() {
    resources::compile();
    gschema::compile();
    desktop::generate();
    metainfo::generate();
    icons::copy();

    let translatable_files = potfiles::collect();
    potfiles::write(&translatable_files);

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        gettext::update_pot(&translatable_files);
        gettext::compile_po();
    }

    #[cfg(windows)]
    windows::embed_manifest();
}
