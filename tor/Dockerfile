FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd
ARG TOR_INTERNAL_ALIAS=tor-internal
ARG TOR_HTTP_TUNNEL_PORT=1080
ARG TOR_DNS_PORT=53
ARG TOR_SOCKS_PORT=9050

ENV TOR_INTERNAL_ALIAS=${TOR_INTERNAL_ALIAS}
ENV TOR_HTTP_TUNNEL_PORT=${TOR_HTTP_TUNNEL_PORT}
ENV TOR_DNS_PORT=${TOR_DNS_PORT}
ENV TOR_SOCKS_PORT=${TOR_SOCKS_PORT}
RUN apk add --no-cache tor
COPY --chown=root:root --chmod=0755 <<-"EOF" /usr/local/bin/torw
    #!/bin/sh
    set -xe
    LISTEN_ADDR="$(nslookup -type=a ${TOR_INTERNAL_ALIAS} | grep -v "127.[0-9]\+.[0-9]\+.[0-9]\+" | awk '/Address:/{print $2}')"
    exec /usr/bin/tor \
        --HTTPTunnelPort "${LISTEN_ADDR}:${TOR_HTTP_TUNNEL_PORT}" \
        --DNSPort "${LISTEN_ADDR}:${TOR_DNS_PORT}" \
        --SocksPort "${LISTEN_ADDR}:${TOR_SOCKS_PORT}" \
        "$@"
EOF
WORKDIR /var/lib/tor
USER tor
EXPOSE ${TOR_HTTP_TUNNEL_PORT}/tcp ${TOR_DNS_PORT}/udp ${TOR_SOCKS_PORT}/tcp
ENTRYPOINT ["/bin/sh", "/usr/local/bin/torw"]
