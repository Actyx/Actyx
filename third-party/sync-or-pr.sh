#!/bin/bash

set -e

SD="$( cd "$( dirname "$0" )" &> /dev/null && /bin/pwd -P )"
echo "script located in $SD"

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"

# Ensure that we are on master, unless --force is given
if [[ "$1" != "--force" ]]; then
    if [[ "$CURRENT_BRANCH" != "master" ]]; then
        echo "error: current branch $CURRENT_BRANCH is not master; this script may only run on master"
        exit 1
    fi
fi


# Get third-party repositories
repos=()
for repo in "$SD"/*; do
    if [ -d "$repo" ]; then
        if test -f "$repo/remote"; then
            echo "found third-party repo $repo"
            repos+=("$repo")
        fi
    fi
done

echo "found ${#repos[@]} third-parties repositories"

LAST_COSMOS_COMMIT=$(git rev-parse --short HEAD)
echo "last cosmos commit: $LAST_COSMOS_COMMIT"

BRANCH_NAME="update-to-cosmos/$LAST_COSMOS_COMMIT"
echo "will create branch $LAST_COSMOS_COMMIT"

SLACK_WEBHOOK="https://hooks.slack.com/services/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"

for repo in "${repos[@]}"
do
    DIR="$repo"
    echo "[$repo] checking third-party repo in $DIR"

    REMOTE=$(cat "$DIR/remote")
    echo "[$repo] using remote $REMOTE"

    # This is the local copy of the actual third-party remote repo
    ACTUAL_DIR_NAME="__tmp_actual"
    ACTUAL_REPO="$DIR/$ACTUAL_DIR_NAME"

    if [ -d "$ACTUAL_REPO" ]; then
       echo "[$repo] pulling remote master since actual repo already found at $ACTUAL_REPO"
        (cd "$ACTUAL_REPO" && git pull origin master)
    else
       echo "[$repo] pulling $REMOTE"
        (cd "$DIR" && gh repo clone "$REMOTE" "$ACTUAL_REPO")
    fi

    echo "[$repo] updating local copy of third-party repo"
    #(cd "$ACTUAL_REPO" && find .  -not -path '.' -not -path './.git/*' -not -path './.git' -print0 | xargs -0 -I {} echo {})
    #(cd "$ACTUAL_REPO" && find .  -not -path '.' -not -path './.git/*' -not -path './.git' -print0 | xargs -0 -I {} rm -rf {})
    rsync -av --progress "$DIR/" "$ACTUAL_REPO/" --exclude .git/ --exclude remote --exclude "$ACTUAL_DIR_NAME" --delete
    #(cd "$DIR/" && find .  -not -path '.' -not -path './__tmp_actual*' -not -path './remote' -print0 | xargs -0 -I {} cp -r {} "$ACTUAL_REPO")

    if [ -z "$(cd $ACTUAL_REPO && git status --porcelain=v1 2>/dev/null)" ]; then
        echo "[$repo] found no differences between cosmos and third-party repo"
    else 
        echo "[$repo] found differences between cosmos and third-party repo"

        echo "[$repo] creating branch $BRANCH_NAME"
        (cd "$ACTUAL_REPO" && git checkout -b "$BRANCH_NAME")

        echo "[$repo] adding changes"
        (cd "$ACTUAL_REPO" && git add --all)

        COMMIT_MSG="updating to Cosmos/$LAST_COSMOS_COMMIT"
        echo "[$repo] creating commit $COMMIT_MSG"
        (cd "$ACTUAL_REPO" && git commit -m "$COMMIT_MSG")

        echo "[$repo] pushing branch to remote"
        (cd "$ACTUAL_REPO" && git push -u origin "$BRANCH_NAME" )

        PR_TITLE="Update to Cosmos/$LAST_COSMOS_COMMIT"
        PR_BODY="Updating to https://github.com/Actyx/Cosmos/commit/$LAST_COSMOS_COMMIT"
        echo "[$repo] creating PR $PR_TITLE"
        PR_URL=$(cd "$ACTUAL_REPO" && gh pr create --title "$PR_TITLE" --body "$PR_BODY" | grep "https://")

        echo "[$repo] opened PR $PR_URL"

        curl -X POST -H "Content-type: application/json" --data "{\"text\":\"Created PR to update $REMOTE to $LAST_COSMOS_COMMIT: $PR_URL\"}" "$SLACK_WEBHOOK"

        #(cd "$ACTUAL_REPO" && git co master)
        rm -rf "$ACTUAL_REPO"

    fi
done

