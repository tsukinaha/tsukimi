cargo_options = ['--manifest-path', meson.project_source_root() / 'crates/tsukimi/Cargo.toml']
cargo_target_dir = meson.project_source_root() / 'target'
cargo_options += ['--target-dir', cargo_target_dir, '--locked']
pkgdatadir = datadir / meson.project_name()

rust_target = get_option('rust-target')
if rust_target == 'release'
    cargo_options += ['--release']
    message('Building in release mode')
else
    message('Building in debug mode')
endif

cargo_bin = cargo_target_dir / rust_target / meson.project_name()

git = find_program('git', required: false)

git_release = false
git_version = version

if git.found()
    tag = run_command(
        git,
        'describe',
        '--tags',
        '--exact-match',
        'HEAD',
        check: false,
    )

    if tag.returncode() == 0
        git_release = true
    else
        describe = run_command(
            git,
            'describe',
            '--tags',
            '--always',
            '--dirty',
            check: false,
        )

        if describe.returncode() == 0
            git_version = describe.stdout().strip()
        endif
    endif
endif

env = environment()

if get_option('sandboxed-build')
    # This is the path used by flatpak-cargo-generator in flatpak-builder-tools
    env.set('CARGO_HOME', meson.project_source_root() / 'cargo')
else
    env.set('CARGO_HOME', meson.project_build_root() / 'cargo-home')
endif

env.set('TSUKIMI_VERSION', version)
env.set('TSUKIMI_LOCALEDIR', localedir)
env.set('TSUKIMI_PKGDATADIR', pkgdatadir)

if git_release
    env.set('TSUKIMI_RELEASE', '1')
else
    env.set('TSUKIMI_GIT_VERSION', git_version)
endif

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
    env: env,
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
