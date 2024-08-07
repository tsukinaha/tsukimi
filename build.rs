use std::process::Command;

fn main() {
    glib_build_tools::compile_resources(
        &["resources/ui"],
        "resources/resources.gresource.xml",
        "tsukimi.gresource",
    );

    #[cfg(target_os = "linux")]
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
}
