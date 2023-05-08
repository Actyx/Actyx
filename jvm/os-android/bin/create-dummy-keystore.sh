#!/bin/bash
# Create a dummy keystore for testing purposes. Used in:
# - .github/workflows/validation.yml

ACTYX_LOCAL="jvm/os-android/actyx-local"

KEYSTORE_NAME="axosandroid.jks"
KEYSTORE_ALIAS="axo"
KEYSTORE_PASSWORD="dummy_password"

mkdir -p $ACTYX_LOCAL app/src/main/assets

keytool \
    -genkey \
    -v \
    -keystore $ACTYX_LOCAL/$KEYSTORE_NAME \
    -alias $KEYSTORE_ALIAS \
    -keyalg RSA \
    -keysize 2048 \
    -validity 7 \
    -keypass $KEYSTORE_PASSWORD \
    -storepass $KEYSTORE_PASSWORD \
    -dname "cn=CN, ou=OU, o=O, c=C"

cat > $ACTYX_LOCAL/actyx.properties <<EOF
ext {
    actyxKeystoreName="../actyx-local/axosandroid.jks"
    actyxKeyAlias="${KEYSTORE_ALIAS}"
    actyxKeystorePassword="${KEYSTORE_PASSWORD}"
    actyxKeyPassword="${KEYSTORE_PASSWORD}"
}
EOF
