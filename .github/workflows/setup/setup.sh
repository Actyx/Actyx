#!/bin/bash
set -e

# Setup everything needed to launch GitHub Actions Runner Agents.

USERNAME="gha"
GROUP_NAME="gh-runners"
REPO_NAME="Actyx"

# readlink's -e flag is not supported on MacOS
if [[ $OSTYPE = "linux-gnu"* ]]; then
    SCRIPT_DIR="$(dirname "$(readlink -e "$0")")"
else
    echo "This script only supports Linux"
    exit 1
fi

help() {
    echo "Usage:"
    echo "  sudo ./setup.sh [-f <RUNNERS_FOLDER_PREFIX>] <ACCESS_TOKEN> <REPO_OWNER> <N_AGENTS>"
    exit 1
}

# Copy the setup scripts and fix the permissions
# This will allow $USERNAME to run the setup scripts through
#   su -c ./script.sh
# In other words: this is the easy & simple way to run setup as another user
copy_setup_scripts () {
    SETUP_FOLDER="/home/$USERNAME/.gha_setup"

    cp -r "$SCRIPT_DIR" "$SETUP_FOLDER"

    chgrp -R $GROUP_NAME "$SETUP_FOLDER"
    chown -R $USERNAME "$SETUP_FOLDER"
    chmod -R g+x "$SETUP_FOLDER"

    cd "$SETUP_FOLDER"
}

# Check for root, required as this script modifies users & systemd
if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
    echo "This script must be run as root!"
    help
fi

# Check if $USERNAME already exists, the aim is to force a clean install
if id "$USERNAME" &>/dev/null; then
    echo "User $USERNAME exists, please delete the user to perform a clean install."
    echo "You can perform the clean-up using:"
    echo "  ./clean_agents.sh [<N_AGENTS>]"
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

# The script only supports 3 arguments
if [[ $# -ne 3 ]]; then
    help
fi

ACCESS_TOKEN=$1
REPO_OWNER=$2
N_AGENTS=$3
if [[ $N_AGENTS -lt 0 ]]; then
    echo "N_AGENTS must be bigger than 0!"
    exit
fi
# Shift arguments out to avoid picking it up further in the script
shift 3


# NOTE: The group was originally created to use along with several users
echo "Setting up group $GROUP_NAME"
getent group $GROUP_NAME >/dev/null || groupadd -g 1997 $GROUP_NAME

# Create a user for the actions as ./config.sh cannot be run from root
# For permissions to (mostly) work properly, the user id needs to be 1000
useradd \
    --uid 1000 \
    -m \
    -g gh-runners \
    -G docker \
    -s /bin/bash \
    "$USERNAME"
usermod -L "$USERNAME"

# This call will `cd` into /home/$USERNAME/.gha_setup, keep that in mind from here onward
copy_setup_scripts
su -c "./setup_dependencies.sh" "$USERNAME"

REGISTRATION_TOKEN=$(curl -L \
    -X POST \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer $ACCESS_TOKEN"\
    -H "X-GitHub-Api-Version: 2022-11-28" \
    https://api.github.com/repos/"$REPO_OWNER"/"$REPO_NAME"/actions/runners/registration-token \
    | jq -r .token
)

# Setup each agent individually, details provided in ./setup_agent.sh
for ((I=1;I<=N_AGENTS;I++)) ; do
    su -c "./setup_agent.sh -f $RUNNERS_FOLDER_PREFIX $I $REPO_OWNER $REPO_NAME $REGISTRATION_TOKEN" "$USERNAME"

    cd "/home/$USERNAME/$RUNNERS_FOLDER_PREFIX-$I"

    # Ensure that the environment snapshot is fresh
    # For more information: https://learn.microsoft.com/en-us/azure/devops/pipelines/agents/linux-agent?view=azure-devops#service-update-environment-variables
    # su -c "./env.sh" "$USERNAME"

    # Install as sudo (no argument) to replicate the Azure Pipelines Setup
    ./svc.sh install "$USERNAME"
    ./svc.sh start
    cd -
done
