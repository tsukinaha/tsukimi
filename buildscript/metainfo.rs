use std::{
    fs,
    path::Path,
    process::Command,
};

const TEMPLATE: &str = "resources/moe.tsuna.tsukimi.metainfo.xml.in";
const OUTPUT: &str = "target/metainfo/moe.tsuna.tsukimi.metainfo.xml";
const PO_DIR: &str = "po";

pub fn generate() {
    println!("cargo:rerun-if-changed={TEMPLATE}");
    println!("cargo:rerun-if-changed={PO_DIR}");

    let out = Path::new(OUTPUT);
    fs::create_dir_all(out.parent().unwrap()).expect("Failed to create target/metainfo");

    let status = Command::new("msgfmt")
        .args(["--xml", "--template", TEMPLATE, "-d", PO_DIR, "-o", OUTPUT])
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!("cargo:warning=msgfmt --xml exited with {s}, falling back to copy");
            fs::copy(TEMPLATE, OUTPUT).expect("Failed to copy metainfo template");
        }
        Err(e) => {
            println!("cargo:warning=msgfmt --xml not available ({e}), falling back to copy");
            fs::copy(TEMPLATE, OUTPUT).expect("Failed to copy metainfo template");
        }
    }

    validate();
}

fn validate() {
    let status = Command::new("appstreamcli")
        .args(["validate", "--no-net", "--explain", OUTPUT])
        .status();

    match status {
        Ok(s) if s.success() => println!("appstreamcli validate: OK"),
        Ok(s) => println!("cargo:warning=appstreamcli validate exited with {s}"),
        Err(_) => println!("cargo:warning=appstreamcli not found, skipping metainfo validation"),
    }
}
