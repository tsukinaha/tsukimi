set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

builddir := "build"
prefix := builddir + "/dev-prefix"

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

update-i18n:
    meson compile -C {{ builddir }} tsukimi-pot
