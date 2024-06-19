FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd
RUN apk add --no-cache curl bind-tools iproute2 fish
COPY --chown=root:root --chmod=0755 dnslookup.fish /usr/local/bin/dnslookup
COPY --chown=root:root --chmod=0755 http_recon.fish /usr/local/bin/http-recon
ENTRYPOINT ["/usr/bin/fish"]
