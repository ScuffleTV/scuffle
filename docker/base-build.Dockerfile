# syntax = docker/dockerfile:1.4
FROM rust:1.66.1-alpine3.17

RUN <<eot
    # Install all dependencies for building the backend and frontend

    # CVEs fixed in 3.0.8-r0
    apk add --no-cache libssl3=3.0.8-r0 libcrypto3=3.0.8-r0 openssl-dev=3.0.8-r0

    # We need to install nodejs to build the frontend
    apk add --no-cache nodejs=18.14.1-r0 yarn=1.22.19-r0

    # We need to install just to use our build script
    apk add --no-cache just=1.8.0-r0 musl-dev=1.2.3-r4 curl=7.87.0-r2

    # Install wasm-pack
    yarn global add wasm-pack

    # Add wasm build target
    rustup target add wasm32-unknown-unknown

    # Clean up cache files
    rm -r /usr/local/cargo/registry
    yarn cache clean
eot
