const PO_FILES: [&str; 1] = [
    "po/zh_CN.po",
];

use std::path::Path;
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
        for po_file in &PO_FILES {
            let po_path = Path::new(po_file);
            let locale = po_path.file_stem().unwrap().to_str().unwrap();
            let mo_file = format!("i18n/locale/{}/LC_MESSAGES/tsukimi.mo", locale);

            let mo_path = Path::new(&mo_file);

            if !mo_path.exists() {
                std::fs::create_dir_all(mo_path.parent().unwrap()).unwrap();
            }

            let status = Command::new("msgfmt")
                .args([po_file, "-o", &mo_file])
                .status()
                .expect("Failed to compile po file");

            if status.success() {
                println!("{}: OK", po_file);
            } else {
                println!("{}: FAILED", po_file);
            }
        }
    }

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=tsukimi-manifest.rc");
        embed_resource::compile("./tsukimi_manifest.rc", embed_resource::NONE);
    }
}
