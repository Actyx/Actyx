#!/bin/bash

latest_stable=`git rev-parse HEAD`

# https://github.com/docker/buildx/issues/1509#issuecomment-1378538197
# https://docs.docker.com/build/attestations/
docker buildx build --provenance=false --load --tag actyx/util:actyx-win-installer-builder-$latest_stable .
docker buildx build --provenance=false --push --tag actyx/util:actyx-win-installer-builder-$latest_stable .
