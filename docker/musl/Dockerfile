# See https://github.com/rust-embedded/cross#supported-targets.
# ARG TARGET=x86_64-unknown-linux-musl
ARG TARGET=armv7-unknown-linux-musleabihf
FROM rustembedded/cross:${TARGET}-0.2.1 as crossbuild-amd64
ARG TARGET
ARG BUILD_RUST_TOOLCHAIN=1.65.0

RUN groupadd -g 1000 builder \
    && useradd -m -u 1000 -g 1000 builder \
    && apt-get update \
    && apt-get install -y liblzma-dev zlib1g-dev protobuf-compiler

RUN mkdir /src && chown -R builder:builder /src

USER builder
# setup rust for current user
RUN curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain $BUILD_RUST_TOOLCHAIN \
    --profile default \
    --target ${TARGET} \
    -y
ENV PATH=/home/builder/.cargo/bin:$PATH

# Install the Rust toolchain
RUN rustup default $BUILD_RUST_TOOLCHAIN

ENV BUILD_RUST_TARGET=$TARGET

# azure pipelines like to be root, at least for image initialization
USER root
WORKDIR /src
