#!/bin/bash
set -ex

# Download and configure a GitHub Actions Runner Agent.
# Warning: This script was not designed to be used standalone!

help() {
    echo "Usage: "
    echo "  ./setup_agent.sh <AGENT_N> <REPO_OWNER> <REPO_NAME> <REGISTRATION_TOKEN>"
    exit
}

if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    echo "This script cannot be run as root!"
    help
fi

if [[ $# -ne 4 ]]; then
    help
fi

# Extract all script inputs
AGENT_N=$1
REPO_OWNER=$2
REPO_NAME=$3
REGISTRATION_TOKEN=$4

RUNNER_FOLDER="gh-runner-$AGENT_N"

mkdir "$HOME/$RUNNER_FOLDER"
# Change directory to the runner folder, keeping track of the previous one
cd "$HOME/$RUNNER_FOLDER"

# Download the runner
RUNNER_VERSION="2.304.0"
wget -qO- "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" | tar xzf -

# Configure the runner
./config.sh \
    --unattended \
    --replace \
    --name gha-"$AGENT_N" \
    --url "https://github.com/${REPO_OWNER}/${REPO_NAME}" \
    --token "${REGISTRATION_TOKEN}"

# shellcheck source=/dev/null
# . "$HOME"/.nvm/nvm.sh

# shellcheck source=/dev/null
source "$HOME/.cargo/env"

./env.sh

# Restore the original directory
cd -
