# syntax=docker/dockerfile:1

FROM docker.io/library/alpine:latest
RUN apk add --no-cache openvpn
WORKDIR /
ENTRYPOINT ["/usr/sbin/openvpn"]
CMD ["--block-ipv6", "--pull-filter", "ignore", "route-ipv6", "--pull-filter", "ignore", "ifconfig-ipv6", "--config", "/etc/openvpn/client/default.ovpn"]
