FROM docker.io/library/alpine:edge@sha256:166710df254975d4a6c4c407c315951c22753dcaa829e020a3fd5d18fff70dd2
RUN echo "@testing http://dl-cdn.alpinelinux.org/alpine/edge/testing" >> /etc/apk/repositories
RUN apk add --no-cache mitmproxy@testing
EXPOSE 1080/tcp
VOLUME ["/data"]
ENTRYPOINT ["/usr/bin/mitmdump", "--mode", "upstream:http://tor:1080", "--listen-host", "0.0.0.0", "--listen-port", "1080", "--save-stream-file", "+/data/session"]
