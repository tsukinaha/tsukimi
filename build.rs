fn main() {
    glib_build_tools::compile_resources(
        &["resources/ui"],
        "resources/resources.gresource.xml",
        "tsukimi.gresource",
    );
}