use std::{
    env,
    fs,
    path::PathBuf,
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/widgets/player_page.blp");
    println!("cargo:rerun-if-changed=resources/resources.gresource.xml");

    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest directory"));
    let input = root.join("src/widgets/player_page.blp");
    let output = root.join("resources/ui/player_page.ui");
    fs::create_dir_all(output.parent().expect("UI output has a parent"))
        .expect("failed to create UI resource directory");

    let status = Command::new("blueprint-compiler")
        .arg("compile")
        .arg("--output")
        .arg(&output)
        .arg(&input)
        .status()
        .expect("failed to run blueprint-compiler; install it and ensure it is in PATH");
    assert!(
        status.success(),
        "blueprint-compiler failed for {}",
        input.display()
    );

    glib_build_tools::compile_resources(
        &[root.join("resources")],
        "resources/resources.gresource.xml",
        "lycoric.gresource",
    );
}
