set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

builddir := "build"
prefix := builddir + "/dev-prefix"
debug_builddir := "build-debug"
debug_prefix := debug_builddir + "/dev-prefix"

default:
    @just --list

setup:
    meson setup {{ builddir }} --prefix "$PWD/{{ prefix }}"

build: setup
    meson compile -C {{ builddir }}

install: build
    meson install -C {{ builddir }}

run *ARGS: install
    cd {{ builddir }} && \
        env \
            GSETTINGS_SCHEMA_DIR="$PWD/dev-prefix/share/glib-2.0/schemas" \
            XDG_DATA_DIRS="$PWD/dev-prefix/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" \
            ./src/tsukimi {{ ARGS }}

setup-debug:
    meson setup {{ debug_builddir }} --prefix "$PWD/{{ debug_prefix }}" -Drust-target=debug

build-debug: setup-debug
    meson compile -C {{ debug_builddir }}

install-debug: build-debug
    meson install -C {{ debug_builddir }}

debug *ARGS: install-debug
    cd {{ debug_builddir }} && \
        env \
            GSETTINGS_SCHEMA_DIR="$PWD/dev-prefix/share/glib-2.0/schemas" \
            XDG_DATA_DIRS="$PWD/dev-prefix/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" \
            ./src/tsukimi {{ ARGS }}
