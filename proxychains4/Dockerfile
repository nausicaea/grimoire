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
ARG PCH_TOR_PROXY_NAME=tailscale
ARG PCH_TOR_PROXY_PORT=9050
ENV PCH_TOR_PROXY_NAME=${PCH_TOR_PROXY_NAME}
ENV PCH_TOR_PROXY_PORT=${PCH_TOR_PROXY_PORT}
COPY --from=builder --chmod=0644 /usr/local/lib/libproxychains4.so /usr/local/lib/libproxychains4.so
COPY --from=builder --chmod=0755 /usr/local/bin/proxychains4 /usr/local/bin/proxychains4
COPY --from=builder --chmod=0755 /usr/local/bin/proxychains4-daemon /usr/local/bin/proxychains4-daemon

RUN apk add --no-cache curl bind-tools

COPY --chown=root:root --chmod=0755 <<-"EOF" /usr/local/bin/proxychains
    #!/bin/sh
    set -ex
    PCH_TOR_PROXY_IP=$(dig +short A ${PCH_TOR_PROXY_NAME})
    printf 'strict_chain\nproxy_dns\nremote_dns_subnet 224\ntcp_read_time_out 1500\ntcp_connect_time_out 8000\n[ProxyList]\nsocks5 %s %s\n' "${PCH_TOR_PROXY_IP}" "${PCH_TOR_PROXY_PORT}" > /tmp/proxychains4.conf
    chmod 0600 /tmp/proxychains4.conf
    exec /usr/local/bin/proxychains4 -f /tmp/proxychains4.conf "$@"
EOF

ENTRYPOINT ["/bin/sh", "/usr/local/bin/proxychains"]
