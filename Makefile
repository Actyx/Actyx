SHELL := /bin/bash

# Git specifics
ifeq ($(origin SYSTEM_PULLREQUEST_SOURCEBRANCH),undefined)
	GIT_BRANCH=$(shell echo $${BUILD_SOURCEBRANCH:-`git rev-parse --abbrev-ref HEAD`} | sed -e "s,refs/heads/,,g")
else
	GIT_BRANCH=$(SYSTEM_PULLREQUEST_SOURCEBRANCH)
endif
ifeq ($(GIT_BRANCH),master)
	git_hash=$(shell git log -1 --pretty=%H)
else
	# Remove the Azure merge commit to get the actual latest commit in the PR branch
	# TODO This will remove all the merge commits, so if the last commit before the merge is also a merge it will get the next-to-last one
	git_hash=$(shell git log -1 --no-merges --pretty=%H)
endif
component=$(shell echo $${DOCKER_TAG:-unknown-x64}|cut -f1 -d-)
arch=$(shell echo $${DOCKER_TAG:-unknown-x64}|cut -f2 -d-)
# These should be moved to the global azure pipelines build
BUILD_RUST_TOOLCHAIN=1.43.1
BUILD_SCCACHE_VERSION=0.2.12

# Build specific
build_dir=dist/build
DOCKER_DIR=ops/docker/images
# The musl images have a separate target so remove them to avoid warnings
DOCKER_BUILD = $(shell arr=(`ls -1 ${DOCKER_DIR} | grep -v -e "^musl\\$$"`); printf "docker-build-%s\n " "$${arr[@]}")
DOCKER_BUILD_SBT = docker-build-bagsapconnector docker-build-flexishttpconnector docker-build-flexisftpconnector docker-build-opcuaconnector docker-build-iafisconnector docker-build-batchcenterconnector

# Helper to try out local builds of Docker images
IMAGE_VERSION:=$(or $(LOCAL_IMAGE_VERSION),latest)

# Debug helpers
print-%	: ; @echo $* = $($*)
debug: print-GIT_BRANCH print-git_hash print-component print-arch print-build_dir

.PHONY: all

all: clean ${DOCKER_BUILD}

clean:
	rm -rf $(build_dir)

docker-login-dockerhub:
	docker login -u $(DOCKERHUB_USER) -p $(DOCKERHUB_PASS)

docker-login: docker-login-dockerhub

DOCKER_REPO ?= actyx/cosmos
getImageNameDockerhub = $(DOCKER_REPO):$(1)-$(2)-$(3)

ifdef RETRY
	RETRY_ONCE = false
else
	RETRY_ONCE = echo && echo "--> RETRYING target $@"; \
               $(MAKE) $(MFLAGS) RETRY=1 $@ || \
               (echo "--> RETRY target $@ FAILED. Continuing..."; true)
endif

# Push to DockerHub
# 1st arg: Name of the docker image
# 2nd arg: tag before the trailing `-<git>` / `-latest`
define fn_docker_push
	$(eval DOCKER_IMAGE_NAME:=$(1))
	$(eval IMAGE_NAME:=$(call getImageNameDockerhub,$(DOCKER_IMAGE_NAME),$(2),$(git_hash)))
	docker push $(IMAGE_NAME)
	$(eval LATEST_IMAGE_TAG:=$(call getImageNameDockerhub,$(DOCKER_IMAGE_NAME),$(2),latest))
	if [ $(GIT_BRANCH) == "master" ]; then \
		docker tag $(IMAGE_NAME) $(LATEST_IMAGE_TAG); \
		docker push $(LATEST_IMAGE_TAG); \
	fi
endef

docker-push-musl: docker-build-musl docker-login
	$(call fn_docker_push,musl,aarch64-unknown-linux-musl)
	$(call fn_docker_push,musl,x86_64-unknown-linux-musl)
	$(call fn_docker_push,musl,armv7-unknown-linux-musleabihf)
	$(call fn_docker_push,musl,arm-unknown-linux-musleabi)

docker-push-%: docker-build-% docker-login
	$(eval DOCKER_IMAGE_NAME:=$(subst docker-push-,,$@))
	$(call fn_docker_push,$(DOCKER_IMAGE_NAME),$(arch))

$(DOCKER_BUILD_SBT): debug clean
	echo "Using sbt-native-packager to generate the docker image..";
	pushd $(SRC_PATH) && \
	sbt validate && \
	IMAGE_NAME=$(call getImageNameDockerhub,$(subst docker-build-,,$@),$(arch),$(git_hash)) sbt docker:publishLocal && \
	popd

# Build the Dockerfile located at `ops/docker/images/musl` for
# the specified $TARGET toolchain.
# 1st arg: Target toolchain
define fn_docker_build_musl
	$(eval TARGET:=$(1))
	$(eval IMAGE_NAME:=$(call getImageNameDockerhub,musl,$(TARGET),$(git_hash)))
	pushd $(DOCKER_DIR)/musl; \
	DOCKER_BUILDKIT=1 docker build -t $(IMAGE_NAME) \
	--build-arg BUILD_RUST_TOOLCHAIN=$(BUILD_RUST_TOOLCHAIN) \
 	--build-arg BUILD_SCCACHE_VERSION=$(BUILD_SCCACHE_VERSION) \
	--build-arg TARGET=$(TARGET) \
	-f Dockerfile .
endef

ifeq ($(arch), aarch64)
DOCKER_BUILD_COMMAND:=buildx build --platform linux/arm64 --load
else
DOCKER_BUILD_COMMAND:=build
endif

${DOCKER_BUILD}: debug clean
	# must not use `component` here because of dependencies
	$(eval DOCKER_IMAGE_NAME:=$(subst docker-build-,,$@))
	mkdir -p $(build_dir)
	cp -RPp $(DOCKER_DIR)/$(DOCKER_IMAGE_NAME)/* $(build_dir)
	$(eval IMAGE_NAME:=$(call getImageNameDockerhub,$(DOCKER_IMAGE_NAME),$(arch),$(git_hash)))
	if [ "$(arch)" == 'armv7hf' ]; then \
		cd $(build_dir); \
		echo "arch is $(arch) - generating Dockerfile using gen-$(arch).sh"; \
		mv Dockerfile Dockerfile-x64; \
		../../ops/docker/gen-$(arch).sh ./Dockerfile-x64 ./Dockerfile; \
	fi
	if [ -f $(build_dir)/prepare-image.sh ]; then \
	 	export ARCH=$(arch); \
		cd $(build_dir); \
		echo 'Running prepare script'; \
		./prepare-image.sh ..; \
	fi

	# requires `qemu-user-static` (ubuntu) package; you might need to restart your docker daemon
	# after setting DOCKER_CLI_EXPERIMENTAL=enabled (or adding `"experimental": "enabled"` to `~/.docker/config.json`)
	# and reset some weird stuff using `docker run --rm --privileged multiarch/qemu-user-static --reset -p yes`
	# to be able to build for `linux/arm64`. (https://github.com/docker/buildx/issues/138)
	DOCKER_CLI_EXPERIMENTAL=enabled DOCKER_BUILDKIT=1 docker $(DOCKER_BUILD_COMMAND) -t $(IMAGE_NAME) \
	--build-arg BUILD_DIR=$(build_dir) \
	--build-arg ARCH=$(arch) \
	--build-arg ARCH_AND_GIT_TAG=$(arch)-$(git_hash) \
	--build-arg IMAGE_NAME=actyx/cosmos \
	--build-arg GIT_COMMIT=$(git_hash) \
	--build-arg GIT_BRANCH=$(GIT_BRANCH) \
	--build-arg BUILD_RUST_TOOLCHAIN=$(BUILD_RUST_TOOLCHAIN) \
	--build-arg BUILD_SCCACHE_VERSION=$(BUILD_SCCACHE_VERSION) \
	-f $(build_dir)/Dockerfile .
	echo "Cleaning up $(build_dir)"
	rm -rf $(build_dir)

docker-build-musl:
	$(call fn_docker_build_musl,aarch64-unknown-linux-musl)
	$(call fn_docker_build_musl,x86_64-unknown-linux-musl)
	$(call fn_docker_build_musl,armv7-unknown-linux-musleabihf)
	$(call fn_docker_build_musl,arm-unknown-linux-musleabi)

docker-build-musl-%:
	$(eval TARGET:=$(subst docker-build-musl-,,$@))
	$(call fn_docker_build_musl,$(TARGET))

docker-build-actyxos: docker-build-docker-logging-plugin

# Build ActyxOS binaries image for the
# specified toolchain.
# 1st arg: output dir (will be created) of the final artifacts
# 2nd arg: target toolchain
# 3rd arg: docker base image
define build_bins_and_move
	$(eval SCCACHE_REDIS?=$(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache))
	mkdir -p $(1)
	docker run -v `pwd`:/src \
	-u builder \
	-w /src/rt-master \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-it $(3) \
	cargo --locked build --release --target $(2) --bins --jobs 8
	find ./rt-master/target/$(2)/release/ -maxdepth 1 -type f -perm -u=x \
		-exec cp {} $(1) \;
	echo "Please find your build artifacts in $(1)."
endef

# Build ActyxOS binaries for Win64
# NOTE: This will only build `ada-cli` and `store-cli`.
# 1st arg: output dir (will be created) of the final artifacts
# 2nd arg: target toolchain
# 3rd arg: docker base image
# actyx-cli depends on fastping-rs depends on libpnet, which needs
# a dependency from winpcap to link against
define build_bins_and_move_win64
	$(eval SCCACHE_REDIS?=$(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache))
	mkdir -p $(1)
	docker run -v `pwd`/rt-master:/src \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-it $(3) \
	bash -c "\
		cd /tmp/ && \
		wget -q https://www.winpcap.org/install/bin/WpdPack_4_1_2.zip && \
		unzip -p WpdPack_4_1_2.zip WpdPack/Lib/x64/Packet.lib > /usr/x86_64-w64-mingw32/lib/Packet.lib && \
		rm WpdPack_4_1_2.zip && \
		cd - && \
		cd actyx-cli && \
		cargo --locked build --release --target $(2) --bin ax --no-default-features --jobs 8 && \
		chown -R builder:builder ../target"
	docker run -v `pwd`/rt-master:/src \
	-u builder \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-it $(3) \
	cargo --locked build --release --target $(2) --bin ada-cli --jobs 8
	docker run -v `pwd`/rt-master:/src \
	-u builder \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-it $(3) \
	cargo --locked build --release --target $(2) --bin store-cli --jobs 8
	find ./rt-master/target/$(2)/release/ -maxdepth 1 -type f -perm -u=x \
		-exec cp {} $(1) \;
	echo "Please find your build artifacts in $(1)."
endef

actyxos-bin-win64: debug clean
	$(eval ARCH?=win64)
	$(eval TARGET:=x86_64-pc-windows-gnu)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(eval IMG:=actyx/util:buildrs-x64-$(IMAGE_VERSION))
	$(call build_bins_and_move_win64,$(OUTPUT),$(TARGET),$(IMG))

actyxos-bin-x64: debug clean
	$(eval ARCH?=x64)
	$(eval TARGET:=x86_64-unknown-linux-musl)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(eval IMG:=actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION))
	$(call build_bins_and_move,$(OUTPUT),$(TARGET),$(IMG))

actyxos-bin-aarch64:
	$(eval ARCH?=aarch64)
	$(eval TARGET:=aarch64-unknown-linux-musl)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(eval IMG:=actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION))
	$(call build_bins_and_move,$(OUTPUT),$(TARGET),$(IMG))

actyxos-bin-armv7hf:
	$(eval ARCH?=armv7hf)
	$(eval TARGET:=armv7-unknown-linux-musleabihf)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(eval IMG:=actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION))
	$(call build_bins_and_move,$(OUTPUT),$(TARGET),$(IMG))

actyxos-bin-arm:
	$(eval ARCH?=arm)
	$(eval TARGET:=arm-unknown-linux-musleabi)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(eval IMG:=actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION))
	$(call build_bins_and_move,$(OUTPUT),$(TARGET),$(IMG))


# Android Shell App, i686 32 bit
android-app: debug
	mkdir -p ./android-shell-app/app/src/main/jniLibs/x86
	cp ./rt-master/target/i686-linux-android/release/libaxstore.so ./android-shell-app/app/src/main/jniLibs/x86/libaxstore.so
	mkdir -p ./android-shell-app/app/src/main/jniLibs/arm64-v8a
	cp ./rt-master/target/aarch64-linux-android/release/libaxstore.so ./android-shell-app/app/src/main/jniLibs/arm64-v8a/libaxstore.so
	./android-shell-app/bin/prepare-gradle-build.sh
	pushd android-shell-app; \
	./gradlew clean ktlint build assemble; \
	popd
	echo 'APK: ./android-shell-app/app/build/outputs/apk/release/app-release.apk'

android: debug clean android-store-lib android-app

android-install: debug
	adb uninstall io.actyx.shell
	adb install ./android-shell-app/app/build/outputs/apk/release/app-release.apk

android-store-lib: debug
	$(call fn-build-android-rust-lib-i686,store-lib)
	$(call fn-build-android-rust-lib-arm64,store-lib)

define fn-build-android-rust-lib
	$(eval TARGET:=$(1))
	$(eval ARCH:=$(2))
	$(eval CRATE:=$(3))
	$(eval SCCACHE_REDIS?=$(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache))
	docker run -v `pwd`:/src \
	-u builder \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-e RUST_BACKTRACE=1 \
	-w /src/rt-master \
	-it actyx/util:buildrs-x64-latest \
	cargo --locked build -p $(CRATE) --lib --release --target $(TARGET) --jobs 8
endef

define fn-build-android-rust-lib-i686
	$(call fn-build-android-rust-lib,i686-linux-android,x86,$(1))
endef

define fn-build-android-rust-lib-arm64
	$(call fn-build-android-rust-lib,aarch64-linux-android,arm64-v8a,$(1))
endef

define fn-build-android-rust-lib-arm
	$(call fn-build-android-rust-lib,armv7-linux-androideabi,armeabi,$(1))
endef

define fn-copy-axosandroid-lib
	$(eval TARGET:=$(1))
	$(eval ARCH:=$(2))
	$(eval LIB:=$(3))
	mkdir -p ./jvm/os-android/app/src/main/jniLibs/$(ARCH)/ && \
		cp ./rt-master/target/$(TARGET)/release/$(LIB).so ./jvm/os-android/app/src/main/jniLibs/$(ARCH)/
endef

define fn-copy-axosandroid-lib-i686
	$(call fn-copy-axosandroid-lib,i686-linux-android,x86,$(1))
endef

define fn-copy-axosandroid-lib-arm64
	$(call fn-copy-axosandroid-lib,aarch64-linux-android,arm64-v8a,$(1))
endef

define fn-copy-axosandroid-lib-arm
	$(call fn-copy-axosandroid-lib,armv7-linux-androideabi,armeabi-v7a,$(1))
endef

# ActyxOS on Android
axosandroid-libs: debug
	$(call fn-build-android-rust-lib-i686,ax-os-node)
	$(call fn-copy-axosandroid-lib-i686,libaxosnode)
	$(call fn-build-android-rust-lib-arm64,ax-os-node)
	$(call fn-copy-axosandroid-lib-arm64,libaxosnode)
	$(call fn-build-android-rust-lib-arm,ax-os-node)
	$(call fn-copy-axosandroid-lib-arm,libaxosnode)

axosandroid-app: debug axosandroid-libs
	./jvm/os-android/bin/get-keystore.sh
	docker run -v `pwd`/jvm/os-android:/src \
	-u builder \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-it actyx/util:buildrs-x64-latest \
	./gradlew clean ktlintCheck build assembleRelease
	echo 'APK: ./jvm/os-android/app/build/outputs/apk/release/app-release.apk'

axosandroid-install: debug
	adb uninstall com.actyx.os.android
	adb install ./jvm/os-android/app/build/outputs/apk/release/app-release.apk

axosandroid: debug clean axosandroid-libs axosandroid-app
