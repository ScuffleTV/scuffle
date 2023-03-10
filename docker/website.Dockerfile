FROM denoland/deno:alpine-1.30.3

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Website Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

# CVEs fixed in 3.0.8-r0
RUN apk add --no-cache libssl3=3.0.8-r0 libcrypto3=3.0.8-r0

COPY frontend/website/server.ts /app/
COPY frontend/website/build /app/build

WORKDIR /app

RUN deno cache --unstable server.ts

STOPSIGNAL SIGINT

USER 1000

ENTRYPOINT ["deno", "run", "--allow-env", "--allow-read", "--allow-net", "server.ts"]
