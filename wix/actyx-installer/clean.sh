#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
rm -rf "$SCRIPT_DIR"/dist
rm -f "$SCRIPT_DIR"/*.msi
rm -f "$SCRIPT_DIR"/*.wixpdb
rm -f "$SCRIPT_DIR"/*.wixobj
