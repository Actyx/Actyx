#!/bin/bash -e

# Refactored from: https://gist.github.com/dopiaza/6449505#gistcomment-2915161

# exit immediately if a command exits with a non-zero status
set -e

# produce failure return code if any command fails in pipe
set -o pipefail

# accepted values: good, danger
alert_type=""
# We get it as input and not as a system var for finer control of final message
stage=""

# colon after var means it has a value rather than it being a bool flag
while getopts 'a:s:c:' OPTION; do
  case "$OPTION" in
    a)
      alert_type="$OPTARG"
      ;;
    s)
      stage="$OPTARG"
      ;;
    c)
      channel="$OPTARG"
      ;;
    ?)
      echo "script usage: $(basename $0) -a {good/danger} -s <BUILD_STEP> -c <ADDITIONAL_CHANNEL>" >&2
      exit 1
      ;;
  esac
done
shift "$(($OPTIND -1))"

# No malicious injections allowed..
stage=$(echo $stage | sed 's/"/\"/g' | sed "s/'/\'/g")

# Remove later
printenv | sort

# Add GitHub link if relevant
if [ "$BUILD_REASON" == "PullRequest" ]; then
  gh_message="PR link: https://github.com/Actyx/Actyx/pull/$SYSTEM_PULLREQUEST_PULLREQUESTNUMBER\n($SYSTEM_PULLREQUEST_SOURCEBRANCH ---> $SYSTEM_PULLREQUEST_TARGETBRANCH)"
else
  gh_message="This is either a build to a \`master\`/\`proj\` branch directly, or a manual build of a PR."
fi

# Modify message on alert type
if [ "$alert_type" == "danger" ]; then
  message="Build $BUILD_BUILDID for $BUILD_SOURCEVERSIONAUTHOR *failed* during *$stage*.\nBuild Link: https://dev.azure.com/ax-ci/Cosmos/_build/results?buildId=$BUILD_BUILDID\n$gh_message"
  # always send failures to rnd, if we're on master
  if [ "$BUILD_SOURCEBRANCHNAME" == "master" ]; then
    channel="$channel #r-n-d"
    message=":ugh: $BUILD_DEFINITIONNAME (master) is *broken*\n$message"
  fi
elif [ "$alert_type" == "good" ]; then
  message="Build $BUILD_BUILDID for $BUILD_SOURCEVERSIONAUTHOR *succeeded*.\nBuild Link: https://dev.azure.com/ax-ci/Cosmos/_build/results?buildId=$BUILD_BUILDID\n$gh_message"
  # always send messages to rnd, if we're on master
  if [ "$BUILD_SOURCEBRANCHNAME" == "master" ]; then
    channel="$channel #r-n-d"
    message=":yay: $BUILD_DEFINITIONNAME (master) is *green*\n$message"
  fi
else
  echo "Alert type provided not allowed: $alert_type"
  exit 1 
fi

# Send to #ci_testing and @user, or fail 
echo "Original Author Of PR: $BUILD_SOURCEVERSIONAUTHOR"
echo "Trying to send notifications to $channel"
for send_channel in $channel
do
	escapedText=$message
  # create JSON payload
  json="{\"channel\": \"$send_channel\", \"username\":\"Actyx Build Notification\", \"icon_emoji\":\"genie\", \"attachments\":[{\"color\":\"$alert_type\" , \"text\": \"$escapedText\"}]}"
  echo "Pinging $send_channel with $message"
  
  # fire off slack message post
  curl -s -d "payload=$json" "https://hooks.slack.com/services/$SLACK_HOOK"

done
