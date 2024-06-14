FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd
ARG TOR_SOCKS_PORT=9050
#ARG TOR_SOCKS_ACCEPT=192.168.215.0/24
ENV TOR_SOCKS_PORT=${TOR_SOCKS_PORT}
RUN apk add --no-cache tor
#COPY <<-EOF /etc/tor/torrc
#    SocksPolicy accept ${TOR_SOCKS_ACCEPT}, reject *
#EOF
COPY --chown=root:root --chmod=0755 <<-"EOF" /usr/local/bin/tor-wrapper
    #!/bin/sh
    IP=$(/usr/bin/awk 'END{print $1}' /etc/hosts)
    exec /usr/bin/tor --SocksPort "$IP:$TOR_SOCKS_PORT" "$@" #--SocksPolicy "accept $IP/24, reject *" "$@"
EOF
WORKDIR /var/lib/tor
USER tor
EXPOSE ${TOR_SOCKS_PORT}/tcp ${TOR_SOCKS_PORT}/udp
ENTRYPOINT ["/bin/sh", "/usr/local/bin/tor-wrapper"]
