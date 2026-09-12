# syntax=docker/dockerfile:1@sha256:ecfaec9ed6d810b56388c508f4121597bfbba70d41a6dfeee4d8cad5f295fc32

FROM docker.io/kalilinux/kali-rolling:latest@sha256:f49124869e4eee549315879c3bd7ef92f23b8753fec7544537bcec1493277096
RUN <<-EOF
    set -ex
    apt-get update
    apt-get install -y kali-linux-headless
    rm -rf /var/lib/apt/lists/*
EOF
ENTRYPOINT ["/usr/bin/bash"]

