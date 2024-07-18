# syntax=docker/dockerfile:1

FROM docker.io/clux/muslrust:1.79.0-stable AS build
ARG ARCH
ARG PACKAGE
ARG AWS_ACCESS_KEY_ID
ARG AWS_SECRET_ACCESS_KEY
ARG SCCACHE_BUCKET
ARG SCCACHE_ENDPOINT
ARG SCCACHE_VERSION=v0.8.1
ARG SCCACHE_CHECKSUM=sha256:452cef732b24415493a7c6bca6e13536eb9464593fa87c753b6b7cb4733e9c50
ARG CARGO_REGISTRY=/root/.cargo/registry
ENV AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
ENV AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
ENV SCCACHE_BUCKET=${SCCACHE_BUCKET}
ENV SCCACHE_ENDPOINT=${SCCACHE_ENDPOINT}
ENV SCCACHE_REGION=auto
ENV SCCACHE_S3_USE_SSL=true
ENV SCCACHE_S3_SERVER_SIDE_ENCRYPTION=true
ENV RUSTC_WRAPPER=/usr/local/bin/sccache
WORKDIR /tmp
ADD --link --checksum=${SCCACHE_CHECKSUM} https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-${ARCH}.tar.gz ./sccache-${SCCACHE_VERSION}-${ARCH}.tar.gz
RUN <<-EOF
set -ex;
tar -xzf ./sccache-${SCCACHE_VERSION}-${ARCH}.tar.gz sccache-${SCCACHE_VERSION}-${ARCH}/sccache;
install -o root -g root -m 0755 ./sccache-${SCCACHE_VERSION}-${ARCH}/sccache /usr/local/bin/sccache;
sccache --show-stats;
EOF
WORKDIR /src
COPY Cargo.toml .
COPY Cargo.lock .
COPY crates ./crates
COPY migrations ./migrations
COPY .sqlx ./.sqlx
RUN cargo fetch --target ${ARCH}
RUN cargo build -p ${PACKAGE} --release --target ${ARCH}

FROM docker.io/library/alpine:latest
ARG ARCH
ARG PACKAGE
COPY --from=build /src/target/$ARCH/release/${PACKAGE} /bin/docker-entrypoint
WORKDIR /
ENTRYPOINT ["/bin/docker-entrypoint"]
