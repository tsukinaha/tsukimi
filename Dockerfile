FROM archlinux:latest as builder

WORKDIR /usr/src/tsukimi

COPY . .
ENV CARGO_TERM_COLOR=always \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    RUST_BACKTRACE=full

RUN pacman -Syu --noconfirm &&\
    pacman -S --noconfirm base-devel gtk4 libadwaita mpv &&\
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y &&\
    mkdir -p $HOME/.local/share/glib-2.0/schemas &&\
    cp moe.tsuna.tsukimi.gschema.xml $HOME/.local/share/glib-2.0/schemas/ &&\
    glib-compile-schemas $HOME/.local/share/glib-2.0/schemas/ &&\
    export PATH=$HOME/.cargo/bin:$PATH &&\
    cargo build --release --locked

RUN pacman -Syu --noconfirm &&\
    pacman -S --noconfirm base-devel gtk4 libadwaita mpv &&\
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y &&\
    mkdir -p $HOME/.local/share/glib-2.0/schemas &&\
    cp moe.tsuna.tsukimi.gschema.xml $HOME/.local/share/glib-2.0/schemas/ &&\
    glib-compile-schemas $HOME/.local/share/glib-2.0/schemas/ &&\
    export PATH=$HOME/.cargo/bin:$PATH &&\
    cargo install cargo-deb --no-default-features &&\
    cargo deb

FROM ubuntu:latest

WORKDIR /usr/src/tsukimi

VOLUME /usr/src/tsukimi

COPY --from=builder /usr/src/tsukimi/target/release/tsukimi /usr/src/tsukimi/

COPY --from=builder /usr/src/tsukimi/target/debian/*.deb /usr/src/tsukimi/

COPY --from=builder root/.local/share/glib-2.0/schemas/gschemas.compiled /usr/src/tsukimi/

ENTRYPOINT ["sleep","3600"]
