FROM scratch

COPY target/x86_64-unknown-linux-musl/release/transcoder /app/

STOPSIGNAL SIGINT

ENTRYPOINT ["/app/transcoder"]
