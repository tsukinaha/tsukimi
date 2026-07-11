%global fallback_version 26.7.3
%global pkg_version %{?version_from_git:%{version_from_git}}%{!?version_from_git:%{fallback_version}}
%global snapshot_release %{?git_snapshot:%{git_snapshot}}%{!?git_snapshot:1}

Name:           tsukimi-git
Version:        %{pkg_version}
Release:        0.%{snapshot_release}%{?dist}
Summary:        GTK4 Jellyfin client for Linux built from the latest Git commit

License:        GPL-3.0-only
URL:            https://github.com/tsukinaha/tsukimi
# For COPR SCM builds with the "make srpm" method, .copr/Makefile generates
# both source archives into the SRPM source directory before rpmbuild -bs.
Source0:        %{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.zst

BuildRequires:  cargo
BuildRequires:  desktop-file-utils
BuildRequires:  gcc
BuildRequires:  gettext
BuildRequires:  meson
BuildRequires:  ninja-build
BuildRequires:  pkgconfig
BuildRequires:  python3
BuildRequires:  rust >= 1.85
BuildRequires:  pkgconfig(dbus-1)
BuildRequires:  pkgconfig(epoxy)
BuildRequires:  pkgconfig(gio-2.0) >= 2.76
BuildRequires:  pkgconfig(glib-2.0) >= 2.76
BuildRequires:  pkgconfig(gstreamer-1.0) >= 1.16
BuildRequires:  pkgconfig(gstreamer-audio-1.0) >= 1.16
BuildRequires:  pkgconfig(gstreamer-bad-audio-1.0) >= 1.16
BuildRequires:  pkgconfig(gstreamer-base-1.0) >= 1.16
BuildRequires:  pkgconfig(gstreamer-play-1.0) >= 1.16
BuildRequires:  pkgconfig(gstreamer-plugins-bad-1.0) >= 1.16
BuildRequires:  pkgconfig(gstreamer-plugins-base-1.0) >= 1.16
BuildRequires:  pkgconfig(gtk4) >= 4.22
BuildRequires:  pkgconfig(libadwaita-1) >= 1.8
BuildRequires:  pkgconfig(mpv) >= 0.38
BuildRequires:  pkgconfig(openssl)

Conflicts:      tsukimi

%description
Tsukimi is a GTK4-based third-party Jellyfin client for Linux. This package is
built from the latest Git commit and is intended for users who want to track
development snapshots instead of tagged releases.

%prep
%autosetup -a 1

%build
%meson \
    -Dsandboxed-build=true \
    -Drust-target=release
%meson_build

%install
%meson_install
%find_lang tsukimi

%files -f tsukimi.lang
%license LICENSE
%doc README.md
%{_bindir}/tsukimi
%{_datadir}/applications/moe.tsuna.tsukimi.desktop
%{_datadir}/glib-2.0/schemas/moe.tsuna.tsukimi.gschema.xml
%{_datadir}/icons/hicolor/scalable/apps/moe.tsuna.tsukimi.svg
%{_datadir}/metainfo/moe.tsuna.tsukimi.metainfo.xml
%{_datadir}/tsukimi/tsukimi.gresource

%changelog
%autochangelog
