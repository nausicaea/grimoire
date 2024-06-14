FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd AS builder
RUN apk add --no-cache build-base
ADD --checksum=sha256:36ddc7f64cb3df2ca4170627c6e0f0dea33d1a6d0730629dff6f5c633f2006f9 https://github.com/rofl0r/proxychains-ng/releases/download/v4.17/proxychains-ng-4.17.tar.xz /tmp/proxychains-ng-4.17.tar.xz
WORKDIR /src
RUN tar -xJf /tmp/proxychains-ng-4.17.tar.xz
WORKDIR /src/proxychains-ng-4.17
RUN <<-EOF
    set -ex
    ./configure --prefix=/usr/local --sysconfdir=/etc
    make
    make install
EOF

FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd
COPY --from=builder --chmod=0644 /usr/local/lib/libproxychains4.so /usr/local/lib/libproxychains4.so
COPY --from=builder --chmod=0755 /usr/local/bin/proxychains4 /usr/local/bin/proxychains4
COPY --from=builder --chmod=0755 /usr/local/bin/proxychains4-daemon /usr/local/bin/proxychains4-daemon
ENTRYPOINT ["/usr/local/bin/proxychains4"]
