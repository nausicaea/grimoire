FROM docker.io/library/debian:stable@sha256:82f8da149d6d567c74564cccd6f355fb5ade42a958e4cde10a1100eaeb24d42e
RUN <<-EOF
    set -ex
    apt-get update
    apt-get install -y openvpn
    rm -rf /var/lib/apt/lists/*
EOF
WORKDIR /
ENTRYPOINT ["/usr/sbin/openvpn"]
CMD ["--block-ipv6", "--pull-filter", "ignore", "route-ipv6", "--pull-filter", "ignore", "ifconfig-ipv6", "--config", "/etc/openvpn/client/default.ovpn"]
