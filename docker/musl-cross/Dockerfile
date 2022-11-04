# README:
# This Dockerfile is mainly an experiment on how we could use BuildKits
# multiplatform capability to replace our 5 dedicated `musl-*` images for
# building. This not used anywhere (CI or via Makefile).
#
# Usage from Cosmos root:
#   docker buildx build \
#     --build-arg TARGET_BINARY=ax \
#     --platform linux/amd64,linux/aarch64 \
#     -f docker/musl-cross/Dockerfile \
#     -o dist .
# This will put the resulting binaries inside `./dist`.
ARG CROSSVER=0.2.1

# Poor man's mapping from buildx arch scheme to cargo
FROM --platform=$BUILDPLATFORM rustembedded/cross:aarch64-unknown-linux-musl-${CROSSVER} AS build-linux-arm64
ENV CARGO_BUILD_TARGET=aarch64-unknown-linux-musl
FROM --platform=$BUILDPLATFORM rustembedded/cross:x86_64-unknown-linux-musl-${CROSSVER} AS build-linux-amd64
ENV CARGO_BUILD_TARGET=x86_64-unknown-linux-musl
FROM --platform=$BUILDPLATFORM rustembedded/cross:arm-unknown-linux-musleabi-${CROSSVER} AS build-linux-armv6
ENV CARGO_BUILD_TARGET=arm-unknown-linux-musleabi
FROM --platform=$BUILDPLATFORM rustembedded/cross:armv7-unknown-linux-musleabihf-${CROSSVER} AS build-linux-armv7
ENV CARGO_BUILD_TARGET=armv7-unknown-linux-musleabihf
FROM --platform=$BUILDPLATFORM rustembedded/cross:x86_64-pc-windows-gnu-${CROSSVER} AS build-windows-amd64
ENV CARGO_BUILD_TARGET=x86_64-pc-windows-gnu

# actual build image
FROM --platform=$BUILDPLATFORM build-${TARGETOS}-${TARGETARCH}${TARGETVARIANT} AS crossbuild
ENV     CARGO_HOME=/usr/local/cargo \
        PATH=/usr/local/cargo/bin:$PATH
ARG RUSTVER=1.65.0

RUN curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain ${RUSTVER} \
    --profile minimal \
    --target ${CARGO_BUILD_TARGET} \
    -y

# actual building
FROM --platform=$BUILDPLATFORM crossbuild AS build
ENV CARGO_BUILD_JOBS=8
ARG TARGETPLATFORM
ARG BUILDPLATFORM
RUN echo "Building on $BUILDPLATFORM for $TARGETPLATFORM"
ARG TARGET_BINARY

WORKDIR /src
COPY rust rust
COPY protocols protocols
WORKDIR /src/rust/actyx
RUN mkdir /src/out

# TODO: add release
# in case a .exe suffix was provided, strip it using `basename`
RUN cargo build \
    --bin `basename $TARGET_BINARY .exe`

# move created binary to a well known place, as there is no access to the
# CARGO_BUILD_TARGET env var in the later extraction stage
# .. would be great to have `cargo build --out-dir` already on stable ..
# TODO add release
RUN mkdir /out && \
    mv /src/rust/actyx/target/$CARGO_BUILD_TARGET/debug/$TARGET_BINARY* /out

# create an empty image just with the built binary to export
FROM scratch
ARG TARGETOS
ARG TARGETARCH

WORKDIR /$TARGETOS/$TARGETARCH

COPY --from=build /out/$TARGET_BINARY* .
