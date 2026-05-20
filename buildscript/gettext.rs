use std::{path::Path, process::Command};

const LINGUAS: &str = include_str!("../po/LINGUAS");


pub fn update_pot(files: &[String]) {
    println!("cargo:rerun-if-changed=po/POTFILES");

    let pkg_version =
        std::env::var("CARGO_PKG_VERSION").unwrap_or_default();

    let rs_files: Vec<&str> = files
        .iter()
        .filter(|f| f.ends_with(".rs"))
        .map(String::as_str)
        .collect();

    let ui_files: Vec<&str> = files
        .iter()
        .filter(|f| f.ends_with(".ui"))
        .map(String::as_str)
        .collect();

    if !rs_files.is_empty() {
        let mut cmd = Command::new("xgettext");
        cmd.args([
            "--package-name=tsukimi",
            &format!("--package-version={pkg_version}"),
            "--from-code=UTF-8",
            "--add-comments",
            // gettextrs function names
            "-kgettext",
            "-kngettext:1,2",
            "-kpgettext:1c,2",
            "-knpgettext:1c,2,3",
            "-o",
            "po/tsukimi.pot",
        ])
        .args(&rs_files);

        run_or_warn(&mut cmd, "xgettext (.rs)");
    }

    if !ui_files.is_empty() {
        let mut cmd = Command::new("xgettext");
        cmd.args([
            "--language=Glade",
            "--from-code=UTF-8",
            "--join-existing",
            "-o",
            "po/tsukimi.pot",
        ])
        .args(&ui_files);

        run_or_warn(&mut cmd, "xgettext (.ui)");
    }
}

pub fn compile_po() {
    for lang in LINGUAS.lines().filter(|l| !l.is_empty()) {
        let po_file = format!("po/{lang}.po");
        let mo_file = format!("i18n/locale/{lang}/LC_MESSAGES/tsukimi.mo");

        println!("cargo:rerun-if-changed={po_file}");

        let mo_path = Path::new(&mo_file);
        if !mo_path.exists() {
            std::fs::create_dir_all(mo_path.parent().unwrap())
                .expect("Failed to create locale directory");
        }

        let status = Command::new("msgfmt")
            .args([&po_file, "-o", &mo_file])
            .status();

        match status {
            Ok(s) if s.success() => println!("{po_file}: OK"),
            Ok(_) => println!("cargo:warning={po_file}: msgfmt failed"),
            Err(e) => println!("cargo:warning=msgfmt not found: {e}"),
        }
    }
}

fn run_or_warn(cmd: &mut Command, label: &str) {
    match cmd.status() {
        Ok(s) if s.success() => {}
        Ok(s) => println!("cargo:warning={label} exited with {s}"),
        Err(e) => println!("cargo:warning={label} not found or failed: {e}"),
    }
}
