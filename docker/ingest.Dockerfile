FROM gcr.io/distroless/static-debian11

COPY target/x86_64-unknown-linux-gnu/release/ingest /app/

STOPSIGNAL SIGINT

USER 1000

ENTRYPOINT ["/app/ingest"]
