#!/bin/bash
set -e

latest_commit=$(git rev-parse HEAD >&1)

echo "Latest commit is: $latest_commit"

docker login
(cd ../ && docker buildx build --load -f ops/docker/images/actyx/Dockerfile -t actyx/cosmos:actyx-${latest_commit} .)

echo "docker image was tagged successfully with: $latest_commit"
