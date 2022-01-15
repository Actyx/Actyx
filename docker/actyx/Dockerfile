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

# actual build image
FROM --platform=$BUILDPLATFORM build-${TARGETOS}-${TARGETARCH}${TARGETVARIANT} AS crossbuild
ENV CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
ARG RUSTVER=1.56.1
ARG ACTYX_VERSION=2.0.0_dev-unknown
ENV ACTYX_VERSION=${ACTYX_VERSION}
ARG CARGO_BUILD_ARGS
ARG AX_PUBLIC_KEY

RUN curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain ${RUSTVER} \
    --profile minimal \
    --target ${CARGO_BUILD_TARGET} \
    -y

# actual building
FROM --platform=$BUILDPLATFORM crossbuild AS build

# all dependencies needed for building
COPY rust rust
COPY protocols protocols
COPY NOTICE NOTICE

RUN cargo install \
    --target ${CARGO_BUILD_TARGET} \
    --path rust/actyx/node \
    --locked \
    ${CARGO_BUILD_ARGS} \
    --bin actyx-linux

# final prod image
FROM --platform=$TARGETPLATFORM alpine:3.13
RUN addgroup -g 1000 -S actyx && \
    adduser -u 1000 -S actyx -G actyx

COPY --from=build --chown=1000:1000 /usr/local/cargo/bin/actyx-linux /usr/bin/actyx

CMD ["actyx", "--working-dir", "/data/actyx-data", "--bind-admin", "0.0.0.0:4458", "--bind-api", "0.0.0.0:4454", "--bind-swarm", "0.0.0.0:4001"]

ARG GIT_COMMIT

RUN mkdir /data && chown actyx /data
USER actyx
VOLUME /data
EXPOSE 4458/TCP
EXPOSE 4454/TCP
EXPOSE 4001/TCP

# https://github.com/opencontainers/image-spec/blob/master/annotations.md
LABEL org.opencontainers.image.title="Actyx" \
    org.opencontainers.image.description="Some description for Actyx" \
    org.opencontainers.image.vendor="Actyx AG" \
    org.opencontainers.image.revision="${GIT_COMMIT}" \
    org.opencontainers.image.created="TODO" \
    org.opencontainers.image.version="TODO" \
    org.opencontainers.image.documentation="https://developer.actyx.com" \
