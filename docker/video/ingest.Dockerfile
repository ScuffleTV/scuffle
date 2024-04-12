FROM ubuntu:lunar

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Video Ingest Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

WORKDIR /app

RUN --mount=type=bind,src=docker/cve.sh,dst=/mount/cve.sh \
    /mount/cve.sh

RUN --mount=type=bind,src=target/release/video-ingest,dst=/mount/video-ingest \
    cp /mount/video-ingest /app/video-ingest && \
    chmod +x /app/video-ingest


STOPSIGNAL SIGTERM

USER 1000

ENTRYPOINT ["/app/video-ingest"]
