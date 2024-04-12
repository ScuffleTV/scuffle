FROM ubuntu:lunar

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Video API Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

WORKDIR /app

RUN --mount=type=bind,src=docker/cve.sh,dst=/mount/cve.sh \
    /mount/cve.sh

RUN --mount=type=bind,src=target/release/video-api,dst=/mount/video-api \
    cp /mount/video-api /app/video-api && \
    chmod +x /app/video-api

STOPSIGNAL SIGTERM

USER 1000

ENTRYPOINT ["/app/video-api"]
