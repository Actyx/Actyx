#!/bin/bash
set -ex

# Download and configure a GitHub Actions Runner Agent.
# Warning: This script was not designed to be used standalone!

help() {
    echo "Usage: "
    echo "  ./setup_agent.sh [-f <RUNNERS_FOLDER_PREFIX>] <AGENT_N> <REPO_OWNER> <REPO_NAME> <REGISTRATION_TOKEN>"
    exit 1
}

if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    echo "This script cannot be run as root!"
    help
fi

RUNNERS_FOLDER_PREFIX="gh-runner"
while getopts 'f:' flag; do
  case "${flag}" in
    f) RUNNERS_FOLDER_PREFIX="${OPTARG}" ;;
    *) error "Unexpected option ${flag}" ;;
  esac
done
# https://stackoverflow.com/a/26295865
shift $((OPTIND-1))

if [[ $# -ne 4 ]]; then
    help
fi

# Extract all script inputs
AGENT_N=$1
REPO_OWNER=$2
REPO_NAME=$3
REGISTRATION_TOKEN=$4

RUNNER_FOLDER="$RUNNERS_FOLDER_PREFIX-$AGENT_N"

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
source "$HOME/.cargo/env"

./env.sh
# https://learn.microsoft.com/en-us/azure/devops/pipelines/agents/linux-agent?view=azure-devops#service-update-environment-variables
echo "VAULT_ADDR=https://vault.actyx.net" >> .env

# Restore the original directory
cd -
