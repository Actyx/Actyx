#!/usr/bin/env bash
## Create actyx.properties file for Gradle build with production
## signing config

mkdir -p jvm/os-android/actyx-local app/src/main/assets

# Ask vault for credentials (in JSON format)
credentials=$(vault kv get --format=json secret/ops.actyx.axosandroid.keystore)

# Dynamically create the file with de-serialized data
cat > jvm/os-android/actyx-local/actyx.properties <<EOF
ext {
    actyxKeystoreName="../actyx-local/axosandroid.jks"
    actyxKeyAlias=$(echo ${credentials} | jq '.data.data.keystore_alias')
    actyxKeystorePassword=$(echo ${credentials} | jq '.data.data.keystore_store_password')
    actyxKeyPassword=$(echo ${credentials} | jq '.data.data.keystore_store_password')
}
EOF

## Create descriptor variable for Gradle build

# First ask vault for credentials
echo ${credentials} | jq -r '.data.data.keystore' | base64 -d > jvm/os-android/actyx-local/axosandroid.jks

# Then dynamically create a decriptor and load it into a variable
descriptor=$(cat <<EOF
{
    "artifacts": [{
        "file" :"actyx-debug.apk",
        "type": "apk"
    },
    {
        "file" :"actyx.apk",
        "type": "apk"
    }],
    "repo": "${BUILD_REPOSITORY_NAME}",
    "commit": "${BUILD_SOURCEVERSION}",
    "timestamp": "$(date -Iseconds)"
}
EOF
)
