FROM archlinux:latest as builder

WORKDIR /usr/src/tsukimi

COPY . .
ENV CARGO_TERM_COLOR=always \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    RUST_BACKTRACE=full

RUN pacman -Syu --noconfirm &&\
    pacman -S --noconfirm base-devel gtk4 libadwaita &&\
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y &&\
    export PATH=$HOME/.cargo/bin:$PATH &&\
    cargo build --release --locked

FROM ubuntu:latest

WORKDIR /usr/src/tsukimi

VOLUME /usr/src/tsukimi

COPY --from=builder /usr/src/tsukimi/target/release/tsukimi /usr/src/tsukimi/

RUN sha256sum tsukimi > tsukimi-x86_64-linux-gnu.sha256sum &&\
    tar -czf tsukimi-x86_64-linux-gnu.tar.gz *

ENTRYPOINT ["sleep","3600"]