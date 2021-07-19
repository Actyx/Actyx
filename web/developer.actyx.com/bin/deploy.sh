#!/bin/bash


set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
SITE_DIR="$SCRIPT_DIR/.."

echo "SITE_DIR=$SITE_DIR"

echo "> Checking if npx is installed..."
(cd "$SITE_DIR" && npx --version)

echo "> Checking if Netlify CLI is installed..."
(cd "$SITE_DIR" && npx netlify --version)

echo "> Checking if NETLIFY_ACCESS_TOKEN is set..."
if [ -z "$NETLIFY_ACCESS_TOKEN" ]; then
    echo "Error: environment variable NETLIFY_ACCESS_TOKEN not set."
    exit 1
else
    echo "Found NETLIFY_ACCESS_TOKEN: ************"
fi

echo "> Checking if NETLIFY_SITE_ID is set..."
if [ -z "$NETLIFY_SITE_ID" ]; then
    echo "Error: environment variable NETLIFY_SITE_ID not set."
    exit 1
else
    echo "Found NETLIFY_SITE_ID: $NETLIFY_SITE_ID"
fi

echo "> Checking if site has been built..."
if [ ! -f "$SITE_DIR/build/index.html" ]; then
    echo "Error: $SITE_DIR/build/index.html not found; has the site been built?"
    exit 1
else
    echo "Found site build"
fi

if [ $# -lt 2 ]; then
    echo "*******************************************************************************"
    echo "** Invalid usage of deploy script. If you are using \`npm run deploy:prod\` or **"
    echo "** \`npm run deploy:draft\`, add a name as shown in the following example:     **"
    echo "** \`npm run deploy:draft -- \"My draft deployment\"\`                           **"
    echo "*******************************************************************************"
    echo "Usage: ./deploy.sh <deploy-type> <deploy-name>"
    exit 1
fi

DEPLOY_TYPE="$1"
DEPLOY_NAME="$2"

if [[ "$DEPLOY_TYPE" != "prod" && "$DEPLOY_TYPE" != "draft" ]]; then
    echo "Error: deploy type must be one of 'prod' or 'draft'"
    exit 1
fi

if [ "$DEPLOY_TYPE" = "prod" ]; then
    echo "> Doing a PRODUCTION deploy (name: $DEPLOY_NAME)"
    (cd "$SITE_DIR" && npx netlify deploy --prod)
else
    echo "> Doing a DRAFT deploy (name: $DEPLOY_NAME)"
    (cd "$SITE_DIR" && npx netlify deploy)
fi



