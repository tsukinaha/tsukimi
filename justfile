set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

builddir := "build"
prefix := builddir + "/dev-prefix"

# ---------------------------------------------------------------------------
# Standard Meson workflow (mutable Fedora, dev containers with host toolchains)
# See docs/build_on_linux.md — this is what upstream documents.
# ---------------------------------------------------------------------------

default:
    @just --list

setup:
    meson setup {{ builddir }} --prefix "$PWD/{{ prefix }}"

compile:
    meson compile -C {{ builddir }}

# First-time configure + compile.
build: setup compile

install: compile
    meson install -C {{ builddir }}

# Run on the host GUI session (requires host runtime libs such as mpv-libs).
run *ARGS: install
    cd {{ builddir }} && \
        env \
            GSETTINGS_SCHEMA_DIR="$PWD/dev-prefix/share/glib-2.0/schemas" \
            XDG_DATA_DIRS="$PWD/dev-prefix/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" \
            TSUKIMI_PKGDATADIR="$PWD/dev-prefix/share/tsukimi" \
            TSUKIMI_LOCALEDIR="$PWD/dev-prefix/share/locale" \
            ./src/tsukimi {{ ARGS }}

update-i18n:
    meson compile -C {{ builddir }} tsukimi-pot

# ---------------------------------------------------------------------------
# Distrobox — local dev on immutable hosts (Bazzite, Silverblue, …)
# Build deps live in the container. Not for COPR, Flatpak, or releases.
# ---------------------------------------------------------------------------

distrobox-name := "tsukimi-build"
distrobox-image := "quay.io/fedora/fedora:latest"

distrobox-run +CMD:
    #!/usr/bin/env bash
    set -euo pipefail
    exec </dev/null
    export CI=1 DEBIAN_FRONTEND=noninteractive DNF_NONINTERACTIVE=1
    if podman container exists "{{ distrobox-name }}" >/dev/null 2>&1 \
        && [[ "$(podman inspect -f '{{{{.State.Running}}}}' "{{ distrobox-name }}" 2>/dev/null || echo false)" == "true" ]]; then
        podman exec -i -w "/run/host${PWD}" "{{ distrobox-name }}" \
            bash -lc 'export CARGO_HOME="$PWD/.cargo-home" CI=1 DEBIAN_FRONTEND=noninteractive; {{ CMD }}'
    else
        distrobox enter --name "{{ distrobox-name }}" --no-tty -- \
            bash -lc 'export CARGO_HOME="$PWD/.cargo-home" CI=1 DEBIAN_FRONTEND=noninteractive; {{ CMD }}'
    fi

distrobox-create:
    distrobox create --name "{{ distrobox-name }}" --image "{{ distrobox-image }}" --yes

distrobox-deps:
    just distrobox-run 'sudo -n dnf install -y \
        gtk4-devel libadwaita-devel wayland-devel \
        gstreamer1-devel gstreamer1-plugins-base-devel gstreamer1-plugins-bad-free-devel \
        openssl-devel gettext-devel libseccomp-devel libepoxy-devel mpv-devel \
        rust cargo meson ninja-build pkgconfig-pkg-config dbus-devel libxml2 \
        desktop-file-utils just'

distrobox-setup:
    just distrobox-run 'meson setup build --prefix "$PWD/build/dev-prefix"'

distrobox-compile:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "[distrobox] compile started: $(date)"
    if pgrep -f "cargo build --manifest-path.*/tsukimi/Cargo.toml" >/dev/null 2>&1 \
        || pgrep -f "rustc --crate-name tsukimi" >/dev/null 2>&1; then
        echo "[distrobox] stale cargo build detected — stopping it (avoids file-lock wait)"
        pkill -f "cargo build --manifest-path.*/tsukimi/Cargo.toml" || true
        pkill -f "rustc --crate-name tsukimi" || true
        pkill -f "ninja -C.*/tsukimi/build" || true
        sleep 2
    fi
    just distrobox-run 'meson compile -C build'
    echo "[distrobox] compile finished: $(date)"

distrobox-install:
    @echo "[distrobox] install started: $(date)"
    just distrobox-run 'meson install -C build'
    @echo "[distrobox] install finished: $(date)"

distrobox-run-app *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    # Drop stray `--` from nested just invocations.
    APP_ARGS=()
    for arg in "$@"; do
        [[ "$arg" == "--" ]] && continue
        APP_ARGS+=("$arg")
    done
    # File check — NOT ldconfig|grep -q (pipefail + SIGPIPE false-negative).
    host_has_libmpv=false
    if [[ -f /lib64/libmpv.so.2 || -f /usr/lib64/libmpv.so.2 ]]; then
        host_has_libmpv=true
    fi
    if [[ "$host_has_libmpv" == "true" ]]; then
        echo "[distrobox] host libmpv found — launching on host (gamepad + display)"
        cd "{{ builddir }}" && \
            env \
                GSK_RENDERER="${GSK_RENDERER:-gl}" \
                GSETTINGS_SCHEMA_DIR="$PWD/dev-prefix/share/glib-2.0/schemas" \
                XDG_DATA_DIRS="$PWD/dev-prefix/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}" \
                TSUKIMI_PKGDATADIR="$PWD/dev-prefix/share/tsukimi" \
                TSUKIMI_LOCALEDIR="$PWD/dev-prefix/share/locale" \
                ./src/tsukimi "${APP_ARGS[@]}"
    else
        echo "[distrobox] no host libmpv — launching in container"
        echo "[distrobox] tip: sudo rpm-ostree install mpv-libs && reboot"
        ARGS="$(printf ' %q' "${APP_ARGS[@]}")"
        just distrobox-run "cd build && env GSK_RENDERER=gl GSETTINGS_SCHEMA_DIR=\"\$PWD/dev-prefix/share/glib-2.0/schemas\" XDG_DATA_DIRS=\"\$PWD/dev-prefix/share:\${XDG_DATA_DIRS:-/usr/local/share:/usr/share}\" TSUKIMI_PKGDATADIR=\"\$PWD/dev-prefix/share/tsukimi\" TSUKIMI_LOCALEDIR=\"\$PWD/dev-prefix/share/locale\" ./src/tsukimi${ARGS}"
    fi
    echo "[distrobox] exited"

# Recompile only (no container recreate, no meson setup).
distrobox-build: distrobox-compile

# compile → install → run
# App flags must follow `--`, e.g. `just dev -- --tv-mode`
dev *ARGS:
    @echo "[distrobox] === dev: compile ==="
    just distrobox-compile
    @echo "[distrobox] === dev: install ==="
    just distrobox-install
    @echo "[distrobox] === dev: launch ==="
    just distrobox-run-app {{ ARGS }}

# Shorthand for TV / gamepad testing.
dev-tv:
    @echo "[distrobox] === dev: compile ==="
    just distrobox-compile
    @echo "[distrobox] === dev: install ==="
    just distrobox-install
    @echo "[distrobox] === dev: launch (tv-mode) ==="
    just distrobox-run-app -- --tv-mode

# One-time bootstrap inside distrobox.
distrobox-init: distrobox-create distrobox-deps distrobox-setup distrobox-compile
