# libmpv-rs
A libmpv abstraction written in rust that's easy to use and provides the ability to read next to all video and audio codecs.

# Dependencies
Rust version >= 1.30. Libmpv version 1.101 (mpv version 0.29.1) is the minimum required version.

For ease of building, you can use the `build_libmpv` feature that is used to link against. Especially useful to cross compile to windows. The `MPV_SOURCE` environment variable needs to be set to a directory containing the mpv source you want to build against. For windows targets this is expected to be already built, with a directory named `MPV_SOURCE/64` or `/32` containing [build artifacts](https://mpv.srsfckn.biz/) for 64-bit and 32-bit targets respectively. On unix this is expected to be a copy of the mpv-build repo.

# Examples
To run an example, execute `cargo run [--release] --example x -- test-data/speech_12kbps_mb.wav`, where x is any of:
* `events`: event enumeration
* `protocol`: implementation of custom `filereader://` protocol that… reads files

# Contributing
All pull requests/issues welcome.
