FROM scratch

COPY target/x86_64-unknown-linux-musl/release/edge /app/

STOPSIGNAL SIGINT

ENTRYPOINT ["/app/edge"]
