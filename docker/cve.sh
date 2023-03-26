set -e

apt-get update 

apt-get install -y --no-install-recommends \
    libgnutls30 \
    libssl3 \
    libsystemd0 \
    libudev1 \
    tar \
    ca-certificates

rm -rf /var/lib/apt/lists/*
