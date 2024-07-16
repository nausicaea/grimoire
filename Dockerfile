# syntax=docker/dockerfile:1

FROM docker.io/library/rust:1.79.0@sha256:4c45f61ebe054560190f232b7d883f174ff287e1a0972c8f6d7ab88da0188870 AS build
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
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked cargo fetch --locked
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked cargo build -p $PACKAGE --release --frozen --offline

FROM docker.io/library/debian:stable-slim@sha256:f8bbfa052db81e5b8ac12e4a1d8310a85d1509d4d0d5579148059c0e8b717d4e
ARG PACKAGE
COPY --from=build /src/target/release/$PACKAGE /bin/docker-entrypoint
WORKDIR /
ENTRYPOINT ["/bin/docker-entrypoint"]
