# Argument needs to be refreshed across several images
# https://docs.docker.com/engine/reference/builder/#scope
# Left without value to force specification on build
ARG TARGET
ARG BUILD_RUST_TOOLCHAIN

# First we need to build GNU Make 4.4.1, see the following link for more information:
# https://github.com/rust-lang/cc-rs/issues/905
FROM ubuntu:20.04 as makebuild
RUN apt-get update && apt-get install -y build-essential curl
RUN curl -L https://ftp.gnu.org/gnu/make/make-4.4.1.tar.gz | tar xz
RUN cd make-4.4.1 && ./configure && make

# See https://github.com/rust-embedded/cross#supported-targets.
ARG TARGET
FROM ghcr.io/cross-rs/${TARGET}:0.2.5 as crossbuild-amd64

ARG TARGET
ARG BUILD_RUST_TOOLCHAIN

COPY --from=makebuild /make-4.4.1/make /usr/bin/make

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
