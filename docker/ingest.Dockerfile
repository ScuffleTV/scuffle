FROM scratch

COPY target/x86_64-unknown-linux-musl/release/ingest /app/

STOPSIGNAL SIGINT

ENTRYPOINT ["/app/ingest"]
