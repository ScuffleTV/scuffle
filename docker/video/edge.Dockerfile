FROM ubuntu:lunar

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Video Edge Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

WORKDIR /app

RUN --mount=type=bind,src=docker/cve.sh,dst=/mount/cve.sh \
    /mount/cve.sh

RUN --mount=type=bind,src=target/release/video-edge,dst=/mount/video-edge \
    cp /mount/video-edge /app/video-edge && \
    chmod +x /app/video-edge

STOPSIGNAL SIGTERM

USER 1000

ENTRYPOINT ["/app/video-edge"]
