# Actyx Dockerfiles

In this folder you'll find the Dockerfiles that are used to build and ship Actyx for various purposes.

- `actyx` - the Dockerfile powering the official Actyx image
- `buildrs` - used to build Actyx for Android
- `musl` - used to build Actyx with [`musl`](https://musl.libc.org/) instead of `glibc`
- `node-manager-win-builder` - used to build Actyx's Node Manager Windows installer

For MacOS, we use a different Dockerfile that requires the MacOSX SDK,
which you will need to get from Apple directly. However, we can show you how we build it.

For reference, we're using MacOSX SDK version 11.1.

<details>
<summary>Dockerfile</summary>

```docker
FROM ubuntu:20.04

ENV TZ=Europe/Berlin
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

RUN apt-get update && apt-get -y install clang cmake git patch python libssl-dev lzma-dev libxml2-dev bash curl
RUN git clone https://github.com/tpoechtrager/osxcross.git

WORKDIR /osxcross
COPY tar/* tarballs/ # this is where the SDKs go
RUN UNATTENDED=1 SDK_VERSION=11.1 ./build.sh
ENV PATH=$PATH:/osxcross/target/bin

RUN groupadd -g 1000 builder && useradd -m -u 1000 -g 1000 builder
USER builder

ARG RUSTVER=1.72.1
ENV CARGO_BUILD_TARGET=x86_64-apple-darwin
RUN curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain ${RUSTVER} \
    --profile minimal \
    --target ${CARGO_BUILD_TARGET} \
    -y
ENV PATH=/home/builder/.cargo/bin:$PATH
ENV CARGO_HOME=/home/builder/.cargo
RUN rustup target add aarch64-apple-darwin

ENV CC_x86_64_apple_darwin="o64-clang"
ENV CXX_x86_64_apple_darwin="o64-clang++"
ENV CC_aarch64_apple_darwin="aarch64-apple-darwin20.2-clang"
ENV CXX_aarch64_apple_darwin="aarch64-apple-darwin20.2-clang++"
RUN echo '[target.x86_64-apple-darwin]\n\
linker = "x86_64-apple-darwin20.2-clang"\n\
ar = "x86_64-apple-darwin20.2-ar"\n\
[target.aarch64-apple-darwin]\n\
linker = "aarch64-apple-darwin20.2-clang"\n\
ar = "aarch64-apple-darwin20.2-ar"' > /home/builder/.cargo/config

USER root
RUN apt-get install -y protobuf-compiler

RUN mkdir /src && chown builder:builder /src
WORKDIR /src

LABEL org.opencontainers.image.source https://github.com/Actyx/osxbuilder
LABEL org.opencontainers.image.description "OSX Builder image for Actyx"
```
<details>
