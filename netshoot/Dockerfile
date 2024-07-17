# syntax=docker/dockerfile:1

FROM docker.io/library/alpine:latest
RUN apk add --no-cache curl bind-tools iproute2 nmap nmap-ncat nmap-nping nmap-nselibs nmap-scripts traceroute tcpdump fish man-db proxychains-ng vim
ENTRYPOINT ["/usr/bin/fish"]
