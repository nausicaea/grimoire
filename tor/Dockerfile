FROM docker.io/kalilinux/kali-rolling:latest@sha256:4bddb69bc2ca5ef2374e1259e5a70164ca53dde93d095560315908fad718f0f7
#FROM docker.io/library/alpine:latest
ARG TOR_SOCKS_PORT=9050
ARG TOR_SOCKS_ACCEPT=192.168.215.0/24
ENV TOR_SOCKS_PORT=${TOR_SOCKS_PORT}
#RUN apk add --no-cache tor
RUN <<-EOF
    set -ex
    apt-get update
    apt-get install -y tor
    rm -rf /var/lib/apt/lists/*
EOF
COPY <<-EOF /etc/tor/torrc
    SocksPort 0.0.0.0:${TOR_SOCKS_PORT}
    SocksPolicy accept ${TOR_SOCKS_ACCEPT}, reject *
EOF
COPY --chown=root:root --chmod=0755 <<-"EOF" /usr/local/sbin/tor-wrapper
    #!/usr/bin/bash
    IP=$(awk 'END{print $1}' /etc/hosts)
    exec /usr/sbin/tor --SocksPort "$IP:$TOR_SOCKS_PORT" "$@"
EOF
RUN <<-EOF
    set -ex
    useradd -r -m -d /var/lib/tor -s /usr/sbin/nologin tor
    chown -R tor:tor /var/lib/tor
    chmod 0750 /var/lib/tor
EOF
WORKDIR /var/lib/tor
USER tor:tor
EXPOSE ${TOR_SOCKS_PORT}/tcp ${TOR_SOCKS_PORT}/udp
ENTRYPOINT ["/usr/bin/bash", "/usr/local/sbin/tor-wrapper"]

