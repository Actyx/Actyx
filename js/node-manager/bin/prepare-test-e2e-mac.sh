#!/bin/bash

# Utility scripts that automatize building and installing the electron app on macOS.
# Use this script before running end-to-end testing `npm run test-e2e`

# From this point on, exit on any errors
set -e

# removed current app if installed
rm -rf "/Applications/Actyx Node Manager.app"

# build application
cd ../
export NVM_DIR=$HOME/.nvm;
source $NVM_DIR/nvm.sh;
nvm use;
npm run make;

# mount app into the file system and visit location
sudo hdiutil attach "./out/make/Actyx Node Manager.dmg"
# unmount volume at the end of the script or in case of errors
trap "sudo hdiutil detach -force \"/Volumes/Actyx Node Manager/\"" EXIT
cd "/Volumes/Actyx Node Manager/"

# install app
cp -r "Actyx Node Manager.app" "/Applications/"
