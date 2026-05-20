pub fn compile() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "tsukimi.gresource",
    );
}
