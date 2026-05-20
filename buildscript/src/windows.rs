pub fn embed_manifest() {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tsukimi_manifest.rc");

    println!("cargo:rerun-if-changed={}", manifest.display());
    embed_resource::compile(manifest, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
