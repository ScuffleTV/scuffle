FROM node:alpine

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Platform Website Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

RUN apk add --upgrade libcrypto3 libssl3 --repository=https://dl-cdn.alpinelinux.org/alpine/edge/community

COPY platform/website/build /app/build
COPY platform/website/entry.js /app/index.js

RUN echo "{\"type\": \"module\"}" > /app/package.json && chown -R 1000:1000 /app

WORKDIR /app

STOPSIGNAL SIGTERM

USER 1000

CMD ["node", "."]
