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

### Fedora COPR

This project is not a good fit for a plain COPR SCM build that only consumes
`tsukimi.spec`, because Fedora may not ship every Rust crate dependency needed
by this application.

For COPR, use the SCM source type together with the `make srpm` method. The
repository provides `.copr/Makefile`, which:

- creates the main source tarball from the Git checkout
- runs `cargo vendor --locked`
- creates a second tarball containing `.cargo/config.toml` and the vendored
  crates
- builds an SRPM that matches `tsukimi.spec`

Recommended COPR settings:

- Source Type: `SCM`
- SCM Method: `make srpm`
- Spec File: `tsukimi.spec`
- Clone URL: this Git repository
- Chroot: Fedora 44+ (`gtk4 >= 4.22`, `libadwaita >= 1.8`)

When the selected SCM checkout points at an exact Git tag such as `v26.6.1`,
`.copr/Makefile` automatically converts that tag into the RPM version
`26.6.1`. If the checkout is not exactly on a tag, it falls back to the
version declared in `Cargo.toml`.

To provide a development snapshot package in the same COPR project, add a
second package source that uses `tsukimi-git.spec`.

- Stable package: `tsukimi` with `Spec File: tsukimi.spec`
- HEAD package: `tsukimi-git` with `Spec File: tsukimi-git.spec`

`tsukimi-git.spec` is intended for building the current HEAD commit. The SRPM
generator injects:

- `Version` from `Cargo.toml`
- `Release` as `0.<YYYYMMDD>git<shortsha>`

This keeps tagged releases and development snapshots separate while still
allowing both packages to live in the same COPR project.
