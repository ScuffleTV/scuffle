set -ex

apt-get update 

apt-get install -y --no-install-recommends \
    libc-bin=2.37-0ubuntu2.2 \
    libc6=2.37-0ubuntu2.2 \
    tar=1.34+dfsg-1.2ubuntu0.2 \
    ca-certificates

apt-get remove --purge -y --allow-remove-essential \
    login \
    passwd

apt clean
rm -rf /var/lib/apt/lists/*
