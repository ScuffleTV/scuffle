# syntax = docker/dockerfile:1.4
FROM rust:1.67.1-alpine3.17

RUN <<eot
    set -eux

    # Install all dependencies for building the backend and frontend

    # CVEs fixed in 3.0.8-r0
    apk add --no-cache libssl3=3.0.8-r0 libcrypto3=3.0.8-r0 openssl-dev=3.0.8-r0

    # We need to install nodejs to build the frontend
    apk add --no-cache nodejs=18.14.1-r0 yarn=1.22.19-r0

    # We need to install just to use our build script
    apk add --no-cache musl-dev=1.2.3-r4 curl=7.87.0-r2 git=2.38.4-r0 tar=1.34-r1 unzip=6.0-r13 zip=3.0-r10 bash=5.2.15-r0

    # Add wasm build target
    rustup target add wasm32-unknown-unknown

    # Install clippy, rustfmt and llvm-tools-preview
    rustup component add clippy rustfmt llvm-tools-preview

    curl https://github.com/jacobdeichert/mask/releases/download/v0.11.3/mask-v0.11.3-x86_64-unknown-linux-musl.zip -L -o /tmp/mask.zip
    unzip /tmp/mask.zip -d /tmp/mask
    mv /tmp/mask/**/mask /usr/local/bin/mask

    rm -r /tmp/mask /tmp/mask.zip

    # Clean up cache files
    rm -r /usr/local/cargo/registry || true
    yarn cache clean
eot
