#!/bin/bash

if (( $# != 1 )); then
    >&2 echo "Provide exactly one argument: The image's base name"
    >&2 echo "Sample usage: $0 actyx/cosmos:os-x86_64"
    exit 2
fi

image_base=$1
current_hash=`git log -1 --pretty=%H`
image_name=$image_base-$current_hash

docker push $image_name
head_of_master=`git rev-parse origin/master`
if [ "$current_hash" == "$head_of_master" ]; then
    latest_image_name=$image_base-latest
    echo "Running on HEAD of master ($head_of_master), tagging image as $latest_image_name .."
    docker tag $image_name $latest_image_name
    docker push $latest_image_name
fi