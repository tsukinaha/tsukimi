use std::{
    path::Path,
    process::Command,
    time::SystemTime,
};

const LINGUAS: &str = include_str!("../../po/LINGUAS");

fn mtime(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn any_newer(sources: &[&str], target: &str) -> bool {
    let target_time = match mtime(target) {
        Some(t) => t,
        None => return true,
    };

    sources
        .iter()
        .any(|s| mtime(s).is_some_and(|t| t > target_time))
}

pub fn update_pot(files: &[String]) {
    println!("cargo:rerun-if-changed=po/POTFILES");

    let pot_file = "po/tsukimi.pot";
    let file_strs: Vec<&str> = files.iter().map(String::as_str).collect();

    if !any_newer(&file_strs, pot_file) {
        println!("{pot_file}: up to date, skipping xgettext");
        return;
    }

    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();

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
            "-kgettext",
            "-kngettext:1,2",
            "-kpgettext:1c,2",
            "-knpgettext:1c,2,3",
            "-o",
            pot_file,
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
            pot_file,
        ])
        .args(&ui_files);

        run_or_warn(&mut cmd, "xgettext (.ui)");
    }
}

pub fn compile_po() {
    for lang in LINGUAS.lines().filter(|l| !l.is_empty()) {
        let po_file = format!("po/{lang}.po");
        let mo_file = format!("target/i18n/locale/{lang}/LC_MESSAGES/tsukimi.mo");

        println!("cargo:rerun-if-changed={po_file}");

        if !any_newer(&[po_file.as_str()], &mo_file) {
            println!("{po_file}: up to date, skipping msgfmt");
            continue;
        }

        let mo_path = Path::new(&mo_file);
        std::fs::create_dir_all(mo_path.parent().unwrap())
            .expect("Failed to create locale directory");

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
