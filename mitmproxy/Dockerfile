# syntax=docker/dockerfile:1

FROM docker.io/library/alpine:edge@sha256:020dfcbaaf4cc1078bf2d9c7ba31a8466e334061dcd2f248001d68f79e52c000
RUN echo "@testing http://dl-cdn.alpinelinux.org/alpine/edge/testing" >> /etc/apk/repositories
RUN apk add --no-cache mitmproxy@testing
EXPOSE 1080/tcp
VOLUME ["/data"]
ENTRYPOINT ["/usr/bin/mitmdump"]
CMD ["--mode", "upstream:http://tor:1080", "--listen-host", "0.0.0.0", "--listen-port", "1080", "--save-stream-file", "+/data/session"]
