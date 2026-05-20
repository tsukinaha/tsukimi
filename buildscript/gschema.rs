use std::{
    path::Path,
    process::Command,
};

const SCHEMA_DIR: &str = "resources";
const OUTPUT_DIR: &str = "target/gschema";

pub fn compile() {
    println!("cargo:rerun-if-changed={SCHEMA_DIR}");

    let out = Path::new(OUTPUT_DIR);
    std::fs::create_dir_all(out).expect("Failed to create target/gschema directory");

    let status = Command::new("glib-compile-schemas")
        .args(["--targetdir", OUTPUT_DIR, SCHEMA_DIR])
        .status();

    match status {
        Ok(s) if s.success() => println!("glib-compile-schemas: OK"),
        Ok(s) => println!("cargo:warning=glib-compile-schemas exited with {s}"),
        Err(e) => println!("cargo:warning=glib-compile-schemas not found or failed: {e}"),
    }
}
