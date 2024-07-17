# syntax=docker/dockerfile:1

FROM docker.io/kalilinux/kali-rolling:latest
RUN <<-EOF
    set -ex
    apt-get update
    apt-get install -y kali-linux-headless
    rm -rf /var/lib/apt/lists/*
EOF
ENTRYPOINT ["/usr/bin/bash"]

