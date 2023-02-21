# syntax = docker/dockerfile:1.4
FROM ubuntu:latest as builder

ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/yarn/bin:$PATH

RUN <<eot
    set -eux

    # Install dependencies
    apt-get update
    apt-get install -y --no-install-recommends \
        libssl3=3.0.2-0ubuntu1.8 \
        libssl-dev=3.0.2-0ubuntu1.8 \
        build-essential \
        zip \
        unzip \
        tar \
        curl \
        git \
        ssh \
        libglib2.0-0 \
        libnss3 \
        libnspr4 \
        libatk1.0-0 \
        libatk-bridge2.0-0 \
        libcups2 \
        libdrm2 \
        libatspi2.0-0 \
        libxcomposite1 \
        libxdamage1 \
        libxfixes3 \
        libxrandr2 \
        libgbm1 \
        libxkbcommon0 \
        libpango-1.0-0 \
        libcairo2 \
        libasound2 \
        gnupg2 \
        ca-certificates

    # Install Node.js
    curl -sL https://deb.nodesource.com/setup_18.x | bash -
    curl -sL https://dl.yarnpkg.com/debian/pubkey.gpg | gpg --dearmor | tee /usr/share/keyrings/yarnkey.gpg >/dev/null
    echo "deb [signed-by=/usr/share/keyrings/yarnkey.gpg] https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list >/dev/null
    apt-get update
    apt-get install -y nodejs yarn --no-install-recommends 

    # Install Yarn
    yarn config set prefix /usr/local/yarn

    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal

    # Install Rust tools
    rustup update
    rustup target add wasm32-unknown-unknown
    rustup component add clippy rustfmt llvm-tools-preview

    cargo install cargo-binstall
    cargo install cargo-watch
    cargo install sqlx-cli --features rustls,postgres --no-default-features
    cargo binstall wasm-pack -y
    cargo binstall cargo-llvm-cov -y
    cargo binstall cargo-nextest -y
    cargo install cargo-audit --features vendored-openssl
    cargo install mask

    # Clean up 
    rm -rf /usr/local/cargo/registry /usr/local/cargo/git 
    apt-get remove -y \
        curl \
        python3 \
        python3.10
    apt-get autoremove -y
    apt-get clean
    rm -rf /var/lib/apt/lists/*

    # Remove SSH host keys, for some reason they are generated on build.
    rm -rf /etc/ssh/ssh_host_* 
eot
