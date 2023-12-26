set -ex

apt-get update
apt-get install -y --no-install-recommends \
        tar \
        make \
        zip \
        unzip \
        curl \
        wget \
        git \
        ssh \
        ca-certificates \
        pkg-config \
        gnupg2 \
        cmake \
        clang-format \
        ninja-build \
        nasm \
        yasm \
        meson \
        libtool \
        autoconf \
        automake \
        build-essential

git clone https://github.com/ScuffleTV/external.git --depth 1 --recurse-submodule --shallow-submodules /tmp/external
/tmp/external/build.sh --prefix /usr/local --build "x264 x265 svt-av1 libvpx opus dav1d ffmpeg opencv"
ldconfig
rm -rf /tmp/external

apt-get remove -y --purge \
        make \
        zip \
        unzip \
        curl \
        wget \
        git \
        ssh \
        ca-certificates \
        pkg-config \
        gnupg2 \
        cmake \
        clang-format \
        ninja-build \
        nasm \
        yasm \
        meson \
        libtool \
        autoconf \
        automake \
        build-essential \
        libpython3.11-stdlib \
        libpython3.11-minimal \
        libpython3.11 \
        python3.11 \
        python3.11-minimal \
        g++ \
        g++-12 \
        gcc \
        gcc-12 \
        "*-dev" \
        "*-dev-*"

apt-get autoremove -y
apt-get clean
rm -rf /var/lib/apt/lists/*
rm /etc/ssh -rf
