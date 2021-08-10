FROM electronuserland/builder:wine-mono
RUN apt-get update && \
    apt-get install --no-install-recommends --assume-yes curl mingw-w64 && \
    apt-get clean



ARG BUILD_RUST_TOOLCHAIN=1.54.0
ENV TARGET=x86_64-pc-windows-gnu
ENV CARGO_BUILD_TARGET=x86_64-pc-windows-gnu

RUN curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain $BUILD_RUST_TOOLCHAIN \
    --profile default \
    --target ${TARGET} \
    -y

ENV PATH=/root/.cargo/bin:$PATH
