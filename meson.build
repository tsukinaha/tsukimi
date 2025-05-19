project('tsukimi', 'rust',
          version: '25.05',
    meson_version: '>= 1.1',
  default_options: [ 'warning_level=2',
                     'werror=false',
                     'buildtype=release'
                   ],
)

major_version = '25'
minor_version = '05'

version = major_version
version += '.' + minor_version

full_version = version

profile = 'Stable'

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
localedir = prefix / get_option('localedir')

datadir = prefix / get_option('datadir')
pkgdatadir = datadir / meson.project_name()
iconsdir = datadir / 'icons'


i18n = import('i18n')

gnome = import('gnome')

dependency('openssl', version: '>= 1.0')
dependency('dbus-1')

dependency('glib-2.0', version: '>= 2.76') # update when changing gtk version
dependency('gio-2.0', version: '>= 2.76') # always same version as glib

dependency('gtk4', version: '>= 4.18.0')
dependency(
  'libadwaita-1', version: '>= 1.6',
  fallback: ['libadwaita', 'libadwaita_dep'],
  default_options: ['tests=false', 'examples=false', 'vapi=false']
)

dependency('mpv', version: '>=0.38')

dependency('gstreamer-1.0', version: '>= 1.16')
dependency('gstreamer-base-1.0', version: '>= 1.16')
dependency('gstreamer-audio-1.0', version: '>= 1.16')
dependency('gstreamer-play-1.0', version: '>= 1.16')
dependency('gstreamer-plugins-base-1.0', version: '>= 1.16')
dependency('gstreamer-plugins-bad-1.0', version: '>= 1.16')
dependency('gstreamer-bad-audio-1.0', version: '>= 1.16')

cargo_sources = files(
  'Cargo.toml',
)

cargo = find_program('cargo', required: true)
cargo_version = run_command(cargo, '--version', check: true).stdout().strip()
message(cargo_version)
rustc_version = run_command('rustc', '--version', check: true).stdout().strip()
message(rustc_version)



gettext_package = meson.project_name()
pkgdatadir = datadir / meson.project_name()

subdir('resources')
subdir('src')
subdir('po')

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)