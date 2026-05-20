use std::{
    fs,
    path::Path,
    process::Command,
};

const TEMPLATE: &str = "resources/moe.tsuna.tsukimi.desktop.in";
const OUTPUT: &str = "target/desktop/moe.tsuna.tsukimi.desktop";
const PO_DIR: &str = "po";

pub fn generate() {
    println!("cargo:rerun-if-changed={TEMPLATE}");
    println!("cargo:rerun-if-changed={PO_DIR}");

    let out = Path::new(OUTPUT);
    fs::create_dir_all(out.parent().unwrap()).expect("Failed to create target/desktop");

    let status = Command::new("msgfmt")
        .args([
            "--desktop",
            "--template",
            TEMPLATE,
            "-d",
            PO_DIR,
            "-o",
            OUTPUT,
        ])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!("cargo:warning=msgfmt --desktop exited with {s}, falling back to copy");
            fs::copy(TEMPLATE, OUTPUT).expect("Failed to copy desktop template");
        }
        Err(e) => {
            println!("cargo:warning=msgfmt --desktop not available ({e}), falling back to copy");
            fs::copy(TEMPLATE, OUTPUT).expect("Failed to copy desktop template");
        }
    }

    validate();
}

fn validate() {
    let status = Command::new("desktop-file-validate").arg(OUTPUT).status();

    match status {
        Ok(s) if s.success() => println!("desktop-file-validate: OK"),
        Ok(s) => println!("cargo:warning=desktop-file-validate exited with {s}"),
        Err(_) => {} // tool not installed, skip silently
    }
}
