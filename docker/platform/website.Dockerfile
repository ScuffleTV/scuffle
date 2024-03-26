FROM nginx:alpine

LABEL org.opencontainers.image.source=https://github.com/scuffletv/scuffle
LABEL org.opencontainers.image.description="Platform Website Container for ScuffleTV"
LABEL org.opencontainers.image.licenses=BSD-4-Clause

COPY docker/platform/website.nginx.conf /etc/nginx/conf.d/default.conf
COPY platform/website/dist /usr/share/nginx/html

STOPSIGNAL SIGTERM

ENTRYPOINT ["nginx", "-g", "daemon off;"]
