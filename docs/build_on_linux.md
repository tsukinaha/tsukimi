## Build on Linux

### Dependencies

Prefer your distribution's packaged `tsukimi` build when it is available. Distro
packages should already provide correct runtime dependencies and resource
installation.

If you want to build Tsukimi from source, see `meson.build` for dependency
requirements. Tsukimi needs relatively new GTK4 and libadwaita releases.

### Development

The repository includes a `justfile` with the common Meson workflows:

```sh
just setup
just build
```

`just setup` configures the local `build/` directory with a development install
prefix under `build/dev-prefix/`. `just build` compiles the GUI program through
Meson.

To run the GUI from the build directory:

```sh
just run
```

The `run` recipe installs into `build/dev-prefix/` first, then launches
`build/src/tsukimi` with `GSETTINGS_SCHEMA_DIR` and `XDG_DATA_DIRS` pointed at
that local install tree so schemas, icons, and bundled resources are found.

To specify debug/release build, run:

```sh
meson setup build --prefix=/usr -Drust-target={debug,release}
meson compile -C build
```
