FROM docker.io/kalilinux/kali-rolling:latest@sha256:4bddb69bc2ca5ef2374e1259e5a70164ca53dde93d095560315908fad718f0f7
RUN <<-EOF
    set -ex
    apt-get update
    #apt-get install -y kali-linux-headless
    #apt-get install -y tor #proxychains4 curl mitmproxy
    #rm -rf /var/lib/apt/lists/*
EOF
ENTRYPOINT ["/usr/bin/bash"]

