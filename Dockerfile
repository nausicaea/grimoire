# syntax=docker/dockerfile:1

FROM docker.io/clux/muslrust:1.79.0-stable AS build
ARG ARCH
ARG PACKAGE

WORKDIR /src
COPY Cargo.toml Cargo.lock .
COPY crates/grimoire/Cargo.toml ./crates/grimoire/Cargo.toml
COPY <<-"EOF" ./crates/grimoire/src/lib.rs
EOF
COPY crates/cert-recon/Cargo.toml ./crates/cert-recon/Cargo.toml
COPY <<-"EOF" ./crates/cert-recon/src/main.rs
fn main() { unimplemented!() }
EOF
COPY crates/dns-recon/Cargo.toml ./crates/dns-recon/Cargo.toml
COPY <<-"EOF" ./crates/dns-recon/src/main.rs
fn main() { unimplemented!() }
EOF
RUN cargo fetch --target $ARCH
COPY crates ./crates
COPY migrations ./migrations
COPY .sqlx ./.sqlx
RUN cargo build -p $PACKAGE --release --target $ARCH

FROM docker.io/library/alpine:latest
ARG ARCH
ARG PACKAGE
COPY --from=build /src/target/$ARCH/release/$PACKAGE /bin/docker-entrypoint
WORKDIR /
ENTRYPOINT ["/bin/docker-entrypoint"]