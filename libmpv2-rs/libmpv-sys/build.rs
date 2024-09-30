use std::env;
use std::path::PathBuf;

#[cfg(not(feature = "use-bindgen"))]
fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let crate_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    std::fs::copy(
        crate_path.join("pregenerated_bindings.rs"),
        out_path.join("bindings.rs"),
    )
    .expect("Couldn't find pregenerated bindings!");

    println!("cargo:rustc-link-lib=mpv");
}

#[cfg(feature = "use-bindgen")]
fn main() {
    let bindings = bindgen::Builder::default()
        .formatter(bindgen::Formatter::Prettyplease)
        .header("include/client.h")
        .header("include/render.h")
        .header("include/render_gl.h")
        .header("include/stream_cb.h")
        .impl_debug(true)
        .opaque_type("mpv_handle")
        .opaque_type("mpv_render_context")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rustc-link-lib=mpv");
}
