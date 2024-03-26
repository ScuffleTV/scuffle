FROM ubuntu:lunar

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Video Transcoder Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

WORKDIR /app

RUN --mount=type=bind,src=docker/ffmpeg.sh,dst=/mount/ffmpeg.sh \
    /mount/ffmpeg.sh

RUN --mount=type=bind,src=docker/cve.sh,dst=/mount/cve.sh \
    /mount/cve.sh

RUN --mount=type=bind,src=target/release/video-transcoder,dst=/mount/video-transcoder \
    cp /mount/video-transcoder /app/video-transcoder && \
    chmod +x /app/video-transcoder

STOPSIGNAL SIGTERM

USER 1000

ENTRYPOINT ["/app/video-transcoder"]
