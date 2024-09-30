#[cfg(feature = "build_libmpv")]
use std::env;

#[cfg(all(feature = "build_libmpv", not(target_os = "windows")))]
use std::process::Command;

#[cfg(not(feature = "build_libmpv"))]
fn main() {}

#[cfg(all(feature = "build_libmpv", target_os = "windows"))]
fn main() {
    let source = env::var("MPV_SOURCE").expect("env var `MPV_SOURCE` not set");

    if env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap() == "64" {
        println!("cargo:rustc-link-search={}/64/", source);
    } else {
        println!("cargo:rustc-link-search={}/32/", source);
    }
}

#[cfg(all(feature = "build_libmpv", not(target_os = "windows")))]
fn main() {
    let source = env::var("MPV_SOURCE").expect("env var `MPV_SOURCE` not set");
    let num_threads = env::var("NUM_JOBS").unwrap();

    // `target` (in cfg) doesn't really mean target. It means target(host) of build script,
    // which is a bit confusing because it means the actual `--target` everywhere else.
    #[cfg(target_pointer_width = "64")]
    {
        if env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap() == "32" {
            panic!("Cross-compiling to different arch not yet supported");
        }
    }
    #[cfg(target_pointer_width = "32")]
    {
        if env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap() == "64" {
            panic!("Cross-compiling to different arch not yet supported");
        }
    }

    // The mpv build script interprets the TARGET env var, which is set by cargo to e.g.
    // x86_64-unknown-linux-gnu, thus the script can't find the compiler.
    // TODO: When Cross-compiling to different archs is implemented, this has to be handled.
    env::remove_var("TARGET");

    let cmd = format!("cd {} && {0}/build -j{}", source, num_threads);

    Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .spawn()
        .expect("mpv-build build failed")
        .wait()
        .expect("mpv-build build failed");

    println!("cargo:rustc-link-search={}/mpv/build/", source);
}
