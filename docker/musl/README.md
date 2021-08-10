# musl build image

This image is used by the top-level Makefile to cross-build our Docker images. It will not be built
automatically. If you update anything in the Dockerfile, please run `build_and_push.sh` to build and
push the images to DockerHub.

To build an image that contains a different Rust version or to add other architectures, edit
`build_and_push.sh`.

