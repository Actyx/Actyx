FROM electronuserland/builder:wine

ARG BUILD_RUST_TOOLCHAIN=1.65.0
ARG NVM_VERSION=0.38.0

RUN apt-get update && \
    apt-get install --no-install-recommends --assume-yes curl mingw-w64 protobuf-compiler && \
    apt-get clean

# Create a new builder user for the rootless setup
RUN useradd --uid 1001 -m -s /bin/bash builder && usermod -L builder && \
    # The original image downloads the wine binaries into /root/.wine,
    # this is incompatible with our "rootless" setup so we need to move it to our user
    # https://askubuntu.com/a/637802
    mv /root/.wine /home/builder/.wine && \
    chown -R builder /home/builder/.wine

USER builder

# Reconfigure wine since we moved the binaries
ENV WINEPREFIX=/home/builder/.wine
RUN winecfg

ENV NVM_DIR=/home/builder/.nvm
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v${NVM_VERSION}/install.sh | bash

ENV TARGET=x86_64-pc-windows-gnu
ENV CARGO_BUILD_TARGET=x86_64-pc-windows-gnu
RUN curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain $BUILD_RUST_TOOLCHAIN \
    --profile default \
    --target ${TARGET} \
    -y

ENV PATH=/home/builder/.cargo/bin:$PATH
