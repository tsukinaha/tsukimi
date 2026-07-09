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
