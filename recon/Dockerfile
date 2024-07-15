FROM docker.io/library/alpine:latest@sha256:77726ef6b57ddf65bb551896826ec38bc3e53f75cdde31354fbffb4f25238ebd
RUN apk add --no-cache curl bind-tools iproute2 fish vim postgresql16-client
COPY --chown=root:root --chmod=0755 cert-recon.fish /usr/local/bin/cert-recon
COPY --chown=root:root --chmod=0755 dns-recon.fish /usr/local/bin/dns-recon
COPY --chown=root:root --chmod=0755 http-recon.fish /usr/local/bin/http-recon
ENTRYPOINT ["/usr/bin/fish"]
