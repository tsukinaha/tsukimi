#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::process::Command;

fn main() {
    glib_build_tools::compile_resources(
        &["resources/ui"],
        "resources/resources.gresource.xml",
        "tsukimi.gresource",
    );
    
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        let po_file = "po/zh_CN.po";
        let mo_file = "i18n/locale/zh_CN/LC_MESSAGES/tsukimi.mo";

        let mo_path = std::path::Path::new(mo_file);

        if !mo_path.exists() {
            std::fs::create_dir_all(mo_path.parent().unwrap()).unwrap();
        }

        let status = Command::new("msgfmt")
            .args([po_file, "-o", mo_file])
            .status()
            .expect("Failed to compile po file");

        if status.success() {
            println!("OK");
        } else {
            println!("FAILED");
        }
    }

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=tsukimi-manifest.rc");
        embed_resource::compile("./tsukimi_manifest.rc", embed_resource::NONE);
    }
}
