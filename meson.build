project(
    'tsukimi',
    'rust',
    version: '26.6.1',
    meson_version: '>= 1.1',
    default_options: ['warning_level=2', 'werror=false', 'buildtype=release'],
)

version = meson.project_version()

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
localedir = prefix / get_option('localedir')
datadir = prefix / get_option('datadir')
pkgdatadir = datadir / meson.project_name()

i18n = import('i18n')

gnome = import('gnome')
python = import('python').find_installation()

cargo_toml = files('Cargo.toml')
crate_min_version_script = '''
import sys
import tomllib

_, cargo_toml, dependency_name = sys.argv

with open(cargo_toml, 'rb') as f:
    dependencies = tomllib.load(f)['dependencies']

features = dependencies[dependency_name]['features']
version_feature = next(feature for feature in features if feature.startswith('v'))
print(version_feature.removeprefix('v').replace('_', '.'))
'''

gtk_min_version = run_command(
    python,
    '-c',
    crate_min_version_script,
    cargo_toml,
    'gtk',
    check: true,
).stdout().strip()

adw_min_version = run_command(
    python,
    '-c',
    crate_min_version_script,
    cargo_toml,
    'adw',
    check: true,
).stdout().strip()

dependency('openssl', version: '>= 1.0')
dependency('dbus-1')

dependency('glib-2.0', version: '>= 2.76')
dependency('gio-2.0', version: '>= 2.76')

dependency('gtk4', version: '>= @0@'.format(gtk_min_version))
dependency(
    'libadwaita-1',
    version: '>= @0@'.format(adw_min_version),
    fallback: ['libadwaita', 'libadwaita_dep'],
    default_options: ['tests=false', 'examples=false', 'vapi=false'],
)

find_program('xmllint', required: false)
dependency('epoxy')
dependency('mpv', version: '>=0.38')

dependency('gstreamer-1.0', version: '>= 1.16')
dependency('gstreamer-base-1.0', version: '>= 1.16')
dependency('gstreamer-audio-1.0', version: '>= 1.16')
dependency('gstreamer-play-1.0', version: '>= 1.16')
dependency('gstreamer-plugins-base-1.0', version: '>= 1.16')
dependency('gstreamer-plugins-bad-1.0', version: '>= 1.16')
dependency('gstreamer-bad-audio-1.0', version: '>= 1.16')

cargo = find_program('cargo', required: true)
cargo_version = run_command(cargo, '--version', check: true).stdout().strip()
message(cargo_version)
rustc_version = run_command('rustc', '--version', check: true).stdout().strip()
message(rustc_version)

subdir('resources')
subdir('src')
subdir('po')

gnome.post_install(
    glib_compile_schemas: true,
    gtk_update_icon_cache: true,
    update_desktop_database: true,
)
