FROM gcr.io/distroless/static-debian11

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Ingest Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

COPY target/x86_64-unknown-linux-gnu/release/ingest /app/

STOPSIGNAL SIGINT

USER 1000

ENTRYPOINT ["/app/ingest"]
