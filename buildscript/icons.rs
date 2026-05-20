use std::{
    fs,
    path::Path,
};

const SRC: &str = "resources/icons/moe.tsuna.tsukimi.png";
const DEST: &str = "target/icons/hicolor/256x256/apps/moe.tsuna.tsukimi.png";

pub fn copy() {
    println!("cargo:rerun-if-changed={SRC}");

    let dest = Path::new(DEST);
    fs::create_dir_all(dest.parent().unwrap()).expect("Failed to create target/icons directory");

    fs::copy(SRC, dest).expect("Failed to copy application icon");
}
