#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

echo "installing dependencies"
(cd "$SCRIPT_DIR/../" && npm i)

echo "running test"
(cd "$SCRIPT_DIR/../" && npm run test)

