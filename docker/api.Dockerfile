FROM scratch

COPY target/x86_64-unknown-linux-musl/release/api /app/

STOPSIGNAL SIGINT

ENTRYPOINT ["/app/api"]
