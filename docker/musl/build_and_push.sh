#!/bin/bash
build_rust_toolchain=1.59.0

# The stable image version is the git commit hash inside `Actyx/Cosmos`, with
# which the respective images was built. Whenever the build images (inside
# ops/docker/images/{buildrs,musl}/Dockerfile) are modified (meaning built and
# pushed), the `Makefile` needs to be changed accordingly.
latest_stable=`git rev-parse HEAD`

for target in armv7-unknown-linux-musleabihf x86_64-unknown-linux-musl aarch64-unknown-linux-musl arm-unknown-linux-musleabi; do
  docker buildx build --load --build-arg BUILD_RUST_TOOLCHAIN=$build_rust_toolchain --build-arg TARGET=$target --tag actyx/util:musl-$target-$latest_stable .
  docker buildx build --push --build-arg BUILD_RUST_TOOLCHAIN=$build_rust_toolchain --build-arg TARGET=$target --tag actyx/util:musl-$target-$latest_stable .
done
