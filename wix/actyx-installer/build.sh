#!/bin/bash

set -e

if [ -z "$1" ]; then
    echo "Usage ./build.sh <actyx-version> <dist-dir> (1=$1, 2=$2)"
    exit 1
fi

if [ -z "$2" ]; then
    echo "Usage ./build.sh <actyx-version> <dist-dir> (1=$1, 2=$2)"
    exit 1
fi

if [ -z "$WIN_CODESIGN_CERTIFICATE" ]; then
    echo "WIN_CODESIGN_CERTIFICATE not set; please set"
    exit 1
fi

if [ -z "$WIN_CODESIGN_PASSWORD" ]; then
    echo "WIN_CODESIGN_PASSWORD not set; please set"
    exit 1
fi

VERSION=$(echo "$1" | sed 's/^\([0-9]*\.[0-9]*\.[0-9]*\).*$/\1/')
echo ": VERSION: $VERSION"

COSMOS_ROOT="/src"
echo ": COSMOS_ROOT: $COSMOS_ROOT"

INSTALLER_SRC="$COSMOS_ROOT/wix/actyx-installer"
echo ": INSTALLER_SRC: $INSTALLER_SRC"

DIST_DIR="$2"
echo ": DIST_DIR: $DIST_DIR"

ACTYX_EXE_PATH="$DIST_DIR/actyx.exe"
echo ": ACTYX_EXE_PATH: $ACTYX_EXE_PATH"

WIX_TOOLSET="$HOME/wix"
echo ": WIX_TOOLSET: $WIX_TOOLSET"
WIX_PROJECT_NAME="project"
echo ": WIX_PROJECT_NAME: $WIX_PROJECT_NAME"

WIX_FILE="$WIX_PROJECT_NAME.wxs"
echo ": WIX_FILE: $WIX_FILE"
WIXOBJ_FILE="$WIX_PROJECT_NAME.wixobj"
echo ": WIXOBJ_FILE: $WIXOBJ_FILE"

CANDLE="wine $WIX_TOOLSET/candle.exe -ext WixUIExtension -ext WixFirewallExtension"
LIGHT="wine $WIX_TOOLSET/light.exe -sval -ext WixUIExtension -ext WixFirewallExtension"

UPGRADE_UUID="AEF9D70D-F219-5E2A-91BD-E11FAEC4BB78"
echo ": UPGRADE_UUID: $UPGRADE_UUID"
ROOT_UUID="846D2068-CCF8-11EB-941E-CF2D82C3B355"
echo ": ROOT_UUID: $ROOT_UUID"

VERSION_UUID=$(python -c "import uuid; print str(uuid.uuid5(uuid.UUID('{$ROOT_UUID}'),'$VERSION')).upper()")
echo ": VERSION_UUID: $VERSION_UUID"

UNSIGNED_INSTALLER_NAME="actyx-x64-unsigned.msi"
echo ": UNSIGNED_INSTALLER_NAME: $UNSIGNED_INSTALLER_NAME"

INSTALLER_NAME="actyx-x64.msi"
echo ": INSTALLER_NAME: $INSTALLER_NAME"

if [[ ! -d $INSTALLER_SRC ]]; then
	echo "error: did not find installer source $INSTALLER_SRC; did you run the docker container correctly?"
	exit 1
fi

(cd "$INSTALLER_SRC" && $CANDLE \
	-dversion="$VERSION" \
	-dversionid="$VERSION_UUID" \
	-dupgradecode="$UPGRADE_UUID" \
	-dactyxexepath="$ACTYX_EXE_PATH" \
	"$WIX_FILE")
(cd "$INSTALLER_SRC" && $LIGHT -out "$UNSIGNED_INSTALLER_NAME" "$WIXOBJ_FILE")

chmod +r "$INSTALLER_SRC/$UNSIGNED_INSTALLER_NAME"

echo "$WIN_CODESIGN_CERTIFICATE" | base64 -di > cert.pfx

echo "Extracting key"
openssl pkcs12 -in cert.pfx -nocerts -nodes -out key.pem -password pass:"$WIN_CODESIGN_PASSWORD"
echo "Extracting certificate"
openssl pkcs12 -in cert.pfx -nokeys -nodes -out cert.pem -password pass:"$WIN_CODESIGN_PASSWORD"
echo "Creating RSA key"
openssl rsa -in key.pem -outform DER -out authenticode.key
echo "Creating cert"
openssl crl2pkcs7 -nocrl -certfile cert.pem -outform DER -out authenticode.spc
echo "Signing MSI"
osslsigncode sign \
	-certs authenticode.spc \
	-key authenticode.key \
	-n "Actyx" \
	-i "http://www.actyx.com/" \
	-t "http://timestamp.digicert.com" \
	-in "$INSTALLER_SRC/$UNSIGNED_INSTALLER_NAME" \
	-out "$INSTALLER_SRC/$INSTALLER_NAME"

mkdir -p "$DIST_DIR"
ls -lap "$DIST_DIR"
cp "$INSTALLER_SRC/$INSTALLER_NAME" "$DIST_DIR"

exit 0
