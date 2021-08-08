#!/bin/bash -x

# Set some base vars for SSH
SSHUSER=ubuntu
SSHHOST=ipfs2.actyx.net
IPNSNAME=axos-android

echo "SYSTEM_PULLREQUEST_TARGETBRANCH: ${SYSTEM_PULLREQUEST_TARGETBRANCH}"
echo "BUILD_SOURCEBRANCH: ${BUILD_SOURCEBRANCH}"

# Go to working directory, and make a folder for all the goodies
cd $(dirname $0)/..
assembly=app/build/outputs/assembly/Actyx/axosandroid/${BUILD_SOURCEVERSION}
echo "Current version's path = $assembly"
mkdir -p ${assembly}
mv app/build/outputs/apk/debug/app-debug.apk ${assembly}/actyx-debug.apk
mv app/build/outputs/apk/release/app-release.apk ${assembly}/actyx.apk

# Make a descriptor for IPFS

apk_version=$(grep versionName app/build.gradle|cut -f2 -d\")

git_log="git log -n 1 --no-decorate"
if [ "$BUILD_REASON" == "PullRequest" ]; then
  # PR build - need to ignore the merge commit
  git_log="$git_log --no-merges"
fi

descriptor=$(cat <<EOF
{
"artifacts": [
{
  "file": "actyx.apk",
  "type": "android",
  "apkVersion": "$apk_version"
},
{
  "file": "actyxos-debug.apk",
  "type": "android",
  "apkVersion": "$apk_version"
}
],
"repo": "Actyx/Internal-Cosmos",
"branch": "$BUILD_SOURCEBRANCH",
"hash": "$($git_log --pretty="%H")",
"commitTime": "$($git_log --pretty="%cI")",
"deployTime": "$(date --iso-8601=seconds)"
}
EOF
)
echo ${descriptor} > ${assembly}/descriptor.json

# Build the connection string for sshpass
ip=$(dig +short A ${SSHHOST})
export SSHPASS=$($HOME/bin/vault write -field=key ssh/creds/ops.actyx.general.all.ipfs2 ip=${ip})

# This will echo the IPFS hash
if [ "${BUILD_SOURCEBRANCH}" == "refs/heads/master" ]; then
    echo "Building master. Upload and publish to IPNS name"
    cmd=ipfs-publish.sh
else
    echo "Building a PR. Upload without publish to IPNS name"
    cmd=ipfs-publish-noipns.sh
fi

tar -cjvf - -C ${assembly} . | sshpass -e ssh ${SSHUSER}@${SSHHOST} ${cmd} ${IPNSNAME} --
