FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd
ARG TOR_SOCKS_LISTEN_ADDR=127.0.0.1
ARG TOR_SOCKS_PORT=9050
ARG TOR_SOCKS_POLICY=accept 127.0.0.0/8, reject *

ENV TOR_SOCKS_LISTEN_ADDR=${TOR_SOCKS_LISTEN_ADDR}
ENV TOR_SOCKS_PORT=${TOR_SOCKS_PORT}
ENV TOR_SOCKS_POLICY=${TOR_SOCKS_POLICY}
RUN apk add --no-cache tor
COPY --chown=root:root --chmod=0755 <<-"EOF" /usr/local/bin/torw
    #!/bin/sh
    set -xe
    exec /usr/bin/tor \
        --SocksPort "${TOR_SOCKS_LISTEN_ADDR}:${TOR_SOCKS_PORT}" \
        --SocksPolicy "${TOR_SOCKS_POLICY}"
        "$@"
EOF
WORKDIR /var/lib/tor
USER tor
EXPOSE ${TOR_SOCKS_PORT}/tcp
ENTRYPOINT ["/bin/sh", "/usr/local/bin/torw"]
