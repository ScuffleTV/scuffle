FROM ubuntu:lunar

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Platform Image Processor Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

WORKDIR /app

RUN --mount=type=bind,src=docker/ffmpeg.sh,dst=/mount/ffmpeg.sh \
    /mount/ffmpeg.sh

RUN --mount=type=bind,src=docker/cve.sh,dst=/mount/cve.sh \
        /mount/cve.sh

RUN --mount=type=bind,src=target/release/platform-image-processor,dst=/mount/platform-image-processor \
        cp /mount/platform-image-processor /app/platform-image-processor && \
        chmod +x /app/platform-image-processor

# STOPSIGNAL SIGTERM

# USER 1000

# ENTRYPOINT ["/app/platform-image-processor"]
