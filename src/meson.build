cargo_options = ['--manifest-path', meson.project_source_root() / 'Cargo.toml']
cargo_target_dir = meson.project_source_root() / 'target'
cargo_options += ['--target-dir', cargo_target_dir]
pkgdatadir = datadir / meson.project_name()

rust_target = get_option('rust-target')
if rust_target == 'release'
    cargo_options += ['--release']
    message('Building in release mode')
else
    message('Building in debug mode')
endif

cargo_bin = cargo_target_dir / rust_target / meson.project_name()

if get_option('sandboxed-build')
    # This is the path used by flatpak-cargo-generator in flatpak-builder-tools
    cargo_env = ['CARGO_HOME=' + meson.project_source_root() / 'cargo']
else
    cargo_env = ['CARGO_HOME=' + meson.project_build_root() / 'cargo-home']
endif

rust_env = cargo_env + [
    'TSUKIMI_VERSION=' + version,
    'TSUKIMI_LOCALEDIR=' + localedir,
    'TSUKIMI_PKGDATADIR=' + pkgdatadir,
]

# We currently don't have rustdoc wiring.
# rustdoc_flags = ' '.join(
#     [
#         '-Zunstable-options',
#         '--enable-index-page',
#         '--extern-html-root-url=gio=https://gtk-rs.org/gtk-rs-core/stable/latest/docs/',
#         '--extern-html-root-url=glib=https://gtk-rs.org/gtk-rs-core/stable/latest/docs/',
#         '--extern-html-root-url=gsk4=https://gtk-rs.org/gtk4-rs/stable/latest/docs/',
#         '--extern-html-root-url=gdk4=https://gtk-rs.org/gtk4-rs/stable/latest/docs/',
#         '--extern-html-root-url=gtk4=https://gtk-rs.org/gtk4-rs/stable/latest/docs/',
#         '--extern-html-root-url=libadwaita=https://world.pages.gitlab.gnome.org/Rust/libadwaita-rs/stable/latest/docs/',
#         '--cfg=docsrs',
#         '-Dwarnings',
#     ],
# )
# doc_env = ['RUSTDOCFLAGS=' + rustdoc_flags]

custom_target(
    'cargo-build',
    build_by_default: true,
    build_always_stale: true,
    output: meson.project_name(),
    console: true,
    env: rust_env,
    install: true,
    install_dir: bindir,
    command: [
        cargo,
        'build',
    ]
    + cargo_options
    + [
        '&&',
        'cp',
        cargo_bin,
        '@OUTPUT@',
    ],
)
