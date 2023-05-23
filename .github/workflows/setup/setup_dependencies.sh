#!/bin/bash
set -e

export NVM_DIR="$HOME/.nvm"

RUNNER_VERSION="${RUNNER_VERSION:-2.304.0}"

help () {
    echo "Usage:"
    echo "  ./setup_dependencies.sh [-n <NVM_VERSION>] [-r <RUST_VERSION>]"
    exit
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

docker buildx create --use

curl -o- "https://raw.githubusercontent.com/nvm-sh/nvm/v${NVM_VERSION}/install.sh" | bash

curl https://sh.rustup.rs -sSf | sh -s -- \
    --default-toolchain "$RUST_VERSION" \
    -y
