FROM denoland/deno:alpine

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Website Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

RUN apk add --upgrade libcrypto3 libssl3 --repository=https://dl-cdn.alpinelinux.org/alpine/edge/community

COPY frontend/website/server.ts /app/
COPY frontend/website/build /app/build

WORKDIR /app

RUN deno cache --unstable server.ts

STOPSIGNAL SIGTERM

USER 1000

ENTRYPOINT ["deno", "run", "--allow-env", "--allow-read", "--allow-net", "server.ts"]
