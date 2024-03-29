#!/bin/bash
build_rust_toolchain=1.72.1

# The stable image version is the git commit hash inside `Actyx/Cosmos`, with
# which the respective images was built. Whenever the build images (inside
# docker/{buildrs,musl}/Dockerfile) are modified (meaning built and
# pushed), the `Makefile` needs to be changed accordingly.
latest_stable=`git rev-parse HEAD`
docker buildx build --push --build-arg BUILD_RUST_TOOLCHAIN=$build_rust_toolchain --tag actyx/util:buildrs-x64-$latest_stable .
