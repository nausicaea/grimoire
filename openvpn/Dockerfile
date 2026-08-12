# syntax=docker/dockerfile:1

FROM docker.io/library/alpine:latest@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b
RUN apk add --no-cache openvpn
WORKDIR /
ENTRYPOINT ["/usr/sbin/openvpn"]
CMD ["--block-ipv6", "--pull-filter", "ignore", "route-ipv6", "--pull-filter", "ignore", "ifconfig-ipv6", "--config", "/etc/openvpn/client/default.ovpn"]
