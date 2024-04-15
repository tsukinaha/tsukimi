extern crate embed_resource;

fn main() {
    glib_build_tools::compile_resources(
        &["resources/ui"],
        "resources/resources.gresource.xml",
        "tsukimi.gresource",
    );
    println!("cargo:rerun-if-changed=tsukimi-manifest.rc");
    embed_resource::compile("./tsukimi-manifest.rc", embed_resource::NONE);
}
