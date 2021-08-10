# Docker image providing rust, cargo, Android SDK, and Android NDK

This image is used by the top-level Makefile to build the Android apps with the appropriate Rust
shared libraries. It will not be built automatically. If you update anything in the Dockerfile,
please run `build_and_push.sh` to build and push the image to DockerHub.

To build an image that contains a different Rust version or to add other architectures, edit
`build_and_push.sh`.

The image is pushed to `actyx/cosmos:buildrs-x64-latest` (https://cloud.docker.com/repository/registry-1.docker.io/actyx/cosmos)

## Usage

From within the folder you want to build, e.g. `actyx/rust`:
```sh
docker run -v $(pwd):/root/src -it actyx/cosmos:buildrs-x64-latest cargo --locked build -p store-lib --release --target i686-linux-android
```
