FROM ubuntu:lunar

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Platform API Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

WORKDIR /app

RUN --mount=type=bind,src=docker/cve.sh,dst=/mount/cve.sh \
    /mount/cve.sh

RUN --mount=type=bind,src=target/release/platform-api,dst=/mount/platform-api \
    cp /mount/platform-api /app/platform-api && \
    chmod +x /app/platform-api

STOPSIGNAL SIGTERM

USER 1000

ENTRYPOINT ["/app/platform-api"]
