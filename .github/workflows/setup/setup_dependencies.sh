#!/bin/bash
set -e

export NVM_DIR="$HOME/.nvm"

RUNNER_VERSION="${RUNNER_VERSION:-2.304.0}"

help () {
    echo "Usage:"
    echo "  ./setup_dependencies.sh [-n <NVM_VERSION>] [-r <RUST_VERSION>]"
    exit 1
}

RUST_VERSION="1.65.0"
NVM_VERSION="0.39.3"

while getopts 'r:n:' flag; do
  case "${flag}" in
    r) RUST_VERSION="${OPTARG}" ;;
    n) NVM_VERSION="${OPTARG}" ;;
    *) error "Unexpected option ${flag}" ;;
  esac
done
# https://stackoverflow.com/a/26295865
shift $((OPTIND-1))


# Creating buildx instances implies creating new containers that stay up and running,
# the following code tries to avoid creating unnecessary instances.

# List buildx instances and check if there is one with `*` (the one in use).
#
# One may extend it with `| awk '{print $1}' | xargs docker buildx inspect`
# and check for the status code and if it's running to get a "proper" check.
#
# For more details, append the commands one by one and check the outputs.
docker buildx ls | grep -E "\*"

# Using $? since we're using pipes and errors don't propagate the same way when using
# pipes + the `if ! cmd;` approach. For more information:
# https://stackoverflow.com/questions/26675681/how-to-check-the-exit-status-using-an-if-statement#comment100308010_26675771
# shellcheck disable=SC2181
if [ $? -ne 0 ] ; then
  docker buildx create --use
fi

curl -o- "https://raw.githubusercontent.com/nvm-sh/nvm/v${NVM_VERSION}/install.sh" | bash

curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain "$RUST_VERSION" \
    -y
