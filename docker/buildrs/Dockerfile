# docker run -v $(pwd):/src -u builder -it actyx/cosmos:build-android-rs-x64-latest cargo --locked build -p store-lib --release --target i686-linux-android
# Supported targets: i686-linux-android, x86_64-linux-android, i686-unknown-linux-gnu (e.g. all necessary dependencies should be there)
# TODO: Build statically linked binaries
# ref https://github.com/Actyx/Cosmos/blob/mca/actyxos-on-arm/docker/actyxos-on-linux/Dockerfile
ARG BUILD_RUST_TOOLCHAIN=1.55.0
FROM rust:${BUILD_RUST_TOOLCHAIN}-buster

ENV RUST_VERSION=1.55.0

ARG BUILD_RUST_TOOLCHAIN

RUN groupadd -g 1000 builder \
  && useradd -m -u 1000 -g 1000 builder

# openjdk-8 for debian buster
RUN apt-get update && \
  apt-get install -y wget gnupg software-properties-common && \
  wget -qO - https://adoptopenjdk.jfrog.io/adoptopenjdk/api/gpg/key/public | apt-key add - && \
  add-apt-repository --yes https://adoptopenjdk.jfrog.io/adoptopenjdk/deb/

RUN apt-get update
RUN apt-get install --no-install-recommends -yq unzip wget cmake \
  # Install jre (adoptopenjdk-8-hotspot-jre) only, if you only need to cross compile for Android
  # We add the JDK here in order to be able to also build the Android app
  adoptopenjdk-8-hotspot adoptopenjdk-8-hotspot-jre \
  # CI deps
  sudo curl jq autossh sshpass dnsutils zip \
  # Necessary libraries
  liblzma-dev zlib1g-dev \
  # win64
  gcc-mingw-w64-x86-64 \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

# make sure this corresponds to the installed jvm version above
ENV JAVA_HOME=/usr/lib/jvm/adoptopenjdk-8-hotspot-amd64/
############################################################################################
# Install Android SDK
ENV ANDROID_SDK_ROOT /opt/android-sdk-linux
RUN mkdir ${ANDROID_SDK_ROOT} && \
  cd ${ANDROID_SDK_ROOT} && \
  wget -q https://dl.google.com/android/repository/sdk-tools-linux-4333796.zip && \
  unzip -q sdk-tools-linux-4333796.zip && \
  rm sdk-tools-linux-4333796.zip && \
  chown -R builder:builder /opt
RUN yes | ${ANDROID_SDK_ROOT}/tools/bin/sdkmanager --licenses | grep -v = || true
RUN yes | ${ANDROID_SDK_ROOT}/tools/bin/sdkmanager "platform-tools" | grep -v = || true
RUN yes | ${ANDROID_SDK_ROOT}/tools/bin/sdkmanager "platforms;android-30" | grep -v = || true
RUN yes | ${ANDROID_SDK_ROOT}/tools/bin/sdkmanager "build-tools;30.0.3"  | grep -v = || true
RUN ${ANDROID_SDK_ROOT}/tools/bin/sdkmanager --update | grep -v = || true

# Install Android NDK
RUN cd /usr/local && \
  wget -q http://dl.google.com/android/repository/android-ndk-r20-linux-x86_64.zip && \
  unzip -q android-ndk-r20-linux-x86_64.zip && \
  rm android-ndk-r20-linux-x86_64.zip
ENV NDK_HOME /usr/local/android-ndk-r20

# Install bundletool
RUN curl -L -o /usr/local/lib/bundletool.jar https://github.com/google/bundletool/releases/download/1.7.1/bundletool-all-1.7.1.jar

# Setup rust specifics with the proper user
USER builder
RUN rustup default ${BUILD_RUST_TOOLCHAIN}
## Android
RUN rustup target add armv7-linux-androideabi
RUN rustup target add aarch64-linux-android
RUN rustup target add i686-linux-android
RUN rustup target add x86_64-linux-android
## Generic
RUN rustup target add x86_64-pc-windows-gnu
RUN rustup target add x86_64-unknown-linux-gnu

# used for ci checks
RUN rustup component add clippy
RUN rustup component add rustfmt

# Set up CC_ and CXX_ env variables for rust cc-rs
ENV CC_x86_64_linux_android="x86_64-linux-android21-clang"
ENV CXX_x86_64_linux_android="x86_64-linux-android21-clang++"
ENV CC_i686_linux_android="i686-linux-android21-clang"
ENV CXX_i686_linux_android="i686-linux-android21-clang++"
ENV CC_aarch64_linux_android="aarch64-linux-android21-clang"
ENV CXX_aarch64_linux_android="aarch64-linux-android21-clang++"
ENV CC_armv7_linux_androideabi="armv7a-linux-androideabi21-clang"
ENV CXX_armv7_linux_androideabi="armv7a-linux-androideabi21-clang++"
ENV CC_x86_64_pc_windows_gnu="x86_64-w64-mingw32-gcc"
ENV CC="cc"
ENV CXX="c++"

# Configure Cargo targets
USER root
RUN mkdir /.cargo
RUN echo '[target.x86_64-linux-android]\n\
  ar = "x86_64-linux-android-ar"\n\
  linker = "x86_64-linux-android21-clang"\n\n\
  [target.i686-linux-android]\n\
  ar = "i686-linux-android-ar"\n\
  linker = "i686-linux-android21-clang"\n\n\
  [target.aarch64-linux-android]\n\
  ar = "aarch64-linux-android-ar"\n\
  linker = "aarch64-linux-android21-clang"\n\n\
  [target.armv7-linux-androideabi]\n\
  ar = "arm-linux-androideabi-ar"\n\
  linker = "armv7a-linux-androideabi21-clang"\n\n\
  [target.x86_64-pc-windows-gnu]\n\
  ar = "x86_64-w64-mingw32-gcc-ar"\n\
  linker = "x86_64-w64-mingw32-gcc"' > /.cargo/config && \
  chown -R builder:builder /.cargo

# Needed so the cargo volume mappings in `Makefile` are correct
ENV CARGO_HOME "/home/builder/.cargo"

# Add toolchains to the path
ENV PATH "${NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin:${PATH}"
RUN echo $PATH

############################################################################################
RUN mkdir /src && chown builder:builder /src
WORKDIR /src

# Make sure the default user is `root`, as azure-pipelines assumes that in order to
# create an appropriate user.
