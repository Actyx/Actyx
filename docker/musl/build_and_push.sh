#!/bin/bash
build_rust_toolchain=1.72.1

# The stable image version is the git commit hash inside `Actyx/Cosmos`, with
# which the respective images was built. Whenever the build images (inside
# ops/docker/images/{buildrs,musl}/Dockerfile) are modified (meaning built and
# pushed), the `Makefile` needs to be changed accordingly.
latest_stable=`git rev-parse HEAD`

# List of supported architectures, source of truth for this is the Makefile
# we're currently only building images for consumption on AMD64 machines
for target in aarch64-unknown-linux-musl x86_64-unknown-linux-musl armv7-unknown-linux-musleabihf; do
  docker buildx build --push --build-arg BUILD_RUST_TOOLCHAIN=$build_rust_toolchain --build-arg TARGET=$target --tag actyx/util:musl-$target-$latest_stable .
done
