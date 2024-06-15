FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd
ARG TOR_SOCKS_POLICY
ARG TOR_SOCKS_PORT
RUN apk add --no-cache tor
COPY --chown=root:root --chmod=0755 <<-EOF /etc/tor/torrc
    SocksPort 0.0.0.0:${TOR_SOCKS_PORT}
    SocksPolicy ${TOR_SOCKS_POLICY}
    SocksPolicy reject *
EOF
WORKDIR /var/lib/tor
USER tor
EXPOSE ${TOR_SOCKS_PORT}/tcp
ENTRYPOINT ["/usr/bin/tor"]
