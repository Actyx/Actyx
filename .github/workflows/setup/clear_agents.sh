#!/bin/bash
set -e

# Script to delete users created by ./setup_agents.sh

USERNAME="gha"

help () {
    echo "Usage:"
    echo "  sudo ./clear_agents.sh <N_AGENTS>"
    exit
}

# Ensure root because of service and user deletion
if [ "${EUID:-$(id -u)}" -ne 0 ]; then
    echo "This script must be run as root!"
    help
fi

# Argument checking, no default provided because this is a destructive operation
if [[ $# -ne 1 ]] ; then
    echo "Number of agents not defined!"
    help
fi

# Check if the $USERNAME exists, just to provide a better error
if ! id "$USERNAME" &>/dev/null; then
    echo "User $USERNAME does not exist, exiting..."
    exit
fi

N_AGENTS="$1"
for ((I=1;I<=N_AGENTS;I++)) ; do
    RUNNER_FOLDER="gh-runner-$I"

    echo "Stopping the runner service..."

    cd /home/$USERNAME/$RUNNER_FOLDER
    ./svc.sh stop
    ./svc.sh uninstall
    cd -
done

echo "Clearing $USERNAME..."
userdel -r $USERNAME
