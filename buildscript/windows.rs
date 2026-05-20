pub fn embed_manifest() {
    println!("cargo:rerun-if-changed=tsukimi_manifest.rc");
    embed_resource::compile("./tsukimi_manifest.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
