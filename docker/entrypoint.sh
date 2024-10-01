#!/bin/bash
set -e

export PATH=/root/.cargo/bin:$PATH

# print $PATH
echo "PATH: $PATH"

# install cargo-deb
cargo install cargo-deb

# print package version
echo "######### PACKAGE VERSION ##########"
cargo --version
echo "cargo-deb version: $(cargo-deb --version)"
echo "gtk4 version: $(pkgconf --modversion gtk4)"
echo "gstreamer version: $(pkgconf --modversion gstreamer-1.0)"
echo "######### PACKAGE VERSION ##########"

# build
cargo b -r --locked

cargo-deb
