# Build on Windows 11

Tsukimi can be built on Windows 11 with the MSYS2 UCRT64 toolchain. The Windows build is packaged as a portable zip that includes the executable, GResource bundle, GSettings schema, GStreamer plugins, and the runtime DLLs needed by GTK/libadwaita/mpv.

## Requirements

Install [MSYS2](https://www.msys2.org/) and open the **UCRT64** shell.

```bash
pacman -Syu
pacman -S --needed \
  git \
  mingw-w64-ucrt-x86_64-cargo \
  mingw-w64-ucrt-x86_64-rust \
  mingw-w64-ucrt-x86_64-pkgconf \
  mingw-w64-ucrt-x86_64-gtk4 \
  mingw-w64-ucrt-x86_64-libadwaita \
  mingw-w64-ucrt-x86_64-gstreamer \
  mingw-w64-ucrt-x86_64-gst-plugins-base \
  mingw-w64-ucrt-x86_64-gst-plugins-good \
  mingw-w64-ucrt-x86_64-gst-plugins-bad \
  mingw-w64-ucrt-x86_64-gst-plugins-ugly \
  mingw-w64-ucrt-x86_64-gst-libav \
  mingw-w64-ucrt-x86_64-mpv \
  mingw-w64-ucrt-x86_64-libepoxy \
  mingw-w64-ucrt-x86_64-gettext \
  mingw-w64-ucrt-x86_64-glib2 \
  mingw-w64-ucrt-x86_64-cc \
  mingw-w64-ucrt-x86_64-lld \
  mingw-w64-ucrt-x86_64-ninja \
  mingw-w64-ucrt-x86_64-meson \
  zip
```

## Build

```bash
git clone https://github.com/tsukinaha/tsukimi
cd tsukimi
mkdir -p secret
echo testing > secret/key
cargo build --release --locked
```

## Runtime resources

For a portable build, compile the resources and schema next to `tsukimi.exe`:

```bash
mkdir -p target/release/share/tsukimi
mkdir -p target/release/share/glib-2.0/schemas

glib-compile-resources \
  --sourcedir=resources \
  --target=target/release/share/tsukimi/tsukimi.gresource \
  resources/resources.gresource.xml

cp resources/moe.tsuna.tsukimi.gschema.xml target/release/share/glib-2.0/schemas/
glib-compile-schemas target/release/share/glib-2.0/schemas
```

Run from the UCRT64 shell:

```bash
GSETTINGS_SCHEMA_DIR="$PWD/target/release/share/glib-2.0/schemas" \
XDG_DATA_DIRS="$PWD/target/release/share" \
GST_PLUGIN_PATH="/ucrt64/lib/gstreamer-1.0" \
target/release/tsukimi.exe
```

## GitHub Actions artifact

The `Windows` workflow builds a portable `tsukimi-x86_64-windows.zip` artifact. Extract it on Windows 11 and launch `tsukimi.cmd`; the script sets `PATH`, `GSETTINGS_SCHEMA_DIR`, `XDG_DATA_DIRS`, and `GST_PLUGIN_PATH` for the bundled runtime.

## Notes

- MPRIS/DBus integration is Linux-only and disabled on Windows.
- The `dmabuf-wayland` mpv output is Linux-only; Windows falls back to the embedded `libmpv` renderer.
- Image cache ETags use extended attributes on Linux and `.etag` sidecar files on Windows.
