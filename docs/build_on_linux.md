## Build on Linux

### Dependencies
- gtk >= 4.14
- mpv >= 0.37
- libadwaita >= 0.5
- gstreamer
- cargo

### With `build.rs`

1. clone repo
```
git clone https://github.com/tsukinaha/tsukimi.git
git submodule update --init --recursive
```
2. compile gschemas
```
mkdir -p $HOME/.local/share/glib-2.0/schemas
cp moe.tsuna.tsukimi.gschema.xml $HOME/.local/share/glib-2.0/schemas/
glib-compile-schemas $HOME/.local/share/glib-2.0/schemas/
```
3. `cargo build --release`
4. install i18n files
```
cp -r "i18n/locale" "${pkgdir}/usr/share/locale"
```


### With Meson
```
meson build
cd build
ninja
ninja install
```