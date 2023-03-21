apt-get update 

apt-get install -y --no-install-recommends \
    libgnutls30=3.7.3-4ubuntu1.2 \
    libssl3=3.0.2-0ubuntu1.8 \
    libsystemd0=249.11-0ubuntu3.7 \
    libudev1=249.11-0ubuntu3.7 \
    tar=1.34+dfsg-1ubuntu0.1.22.04.1 \
    ca-certificates \

rm -rf /var/lib/apt/lists/*
