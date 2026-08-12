# syntax=docker/dockerfile:1

FROM docker.io/library/alpine:latest@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b
RUN apk add --no-cache curl bind-tools iproute2 nmap nmap-ncat nmap-nping nmap-nselibs nmap-scripts traceroute tcpdump fish man-db proxychains-ng vim
ENTRYPOINT ["/usr/bin/fish"]
