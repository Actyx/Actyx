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
BUILD_RUST_TOOLCHAIN=1.38.0
BUILD_SCCACHE_VERSION=0.2.12

# Build specific
build_dir=dist/build
DOCKER_DIR=ops/docker/images
DOCKER_BUILD = $(shell arr=(`ls ${DOCKER_DIR}`); printf "docker-build-%s\n " "$${arr[@]}")
DOCKER_BUILD_SBT = docker-build-bagsapconnector docker-build-flexishttpconnector docker-build-flexisftpconnector docker-build-opcuaconnector docker-build-iafisconnector docker-build-batchcenterconnector

# Debug helpers
print-%  : ; @echo $* = $($*)
debug: print-GIT_BRANCH print-git_hash print-component print-arch print-build_dir

.PHONY: all

all: clean ${DOCKER_BUILD}

clean:
	rm -rf $(build_dir)

docker-login-github:
	docker login docker.pkg.github.com -u $(GITHUB_PKG_USER) -p $(GITHUB_PKG_PASS)

docker-login: docker-login-github

getImageName = docker.pkg.github.com/actyx/cosmos/$(1):$(2)-$(3)

ifdef RETRY
	RETRY_ONCE = false
else
	RETRY_ONCE = echo && echo "--> RETRYING target $@"; \
               $(MAKE) $(MFLAGS) RETRY=1 $@ || \
               (echo "--> RETRY target $@ FAILED. Continuing..."; true)
endif

docker-push-%: docker-build-% docker-login
	$(eval DOCKER_IMAGE_NAME:=$(subst docker-push-,,$@))
	$(eval IMAGE_NAME:=$(call getImageName,$(DOCKER_IMAGE_NAME),$(arch),$(git_hash)))
	# docker push sometimes fails because of the remote registry
	# this started to happen more often as we switched to GitHub Package Registry
	docker push $(IMAGE_NAME) || $(RETRY_ONCE)
	$(eval LATEST_IMAGE_TAG:=$(call getImageName,$(DOCKER_IMAGE_NAME),$(arch),latest))
	if [ $(GIT_BRANCH) == "master" ]; then \
	  docker tag $(IMAGE_NAME) $(LATEST_IMAGE_TAG); \
		docker push $(LATEST_IMAGE_TAG); \
	fi

$(DOCKER_BUILD_SBT): debug clean
	$(eval DOCKER_IMAGE_NAME:=$(subst docker-build-,,$@))
	$(eval IMAGE_NAME:=$(call getImageName,$(DOCKER_IMAGE_NAME),$(arch),$(git_hash)))
	echo "Using sbt-native-packager to generate the docker image..";
	pushd $(SRC_PATH); \
	IMAGE_NAME=$(IMAGE_NAME) sbt docker:publishLocal; \
	popd

${DOCKER_BUILD}: debug clean
	# must not use `component` here because of dependencies
	$(eval DOCKER_IMAGE_NAME:=$(subst docker-build-,,$@))
	$(eval IMAGE_NAME:=$(call getImageName,$(DOCKER_IMAGE_NAME),$(arch),$(git_hash)))
	mkdir -p $(build_dir)
	cp -RPp $(DOCKER_DIR)/$(DOCKER_IMAGE_NAME)/* $(build_dir)
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
	DOCKER_BUILDKIT=1 docker build -t $(IMAGE_NAME) \
	--build-arg BUILD_DIR=$(build_dir) \
	--build-arg ARCH=$(arch) \
	--build-arg ARCH_AND_GIT_TAG=$(arch)-$(git_hash) \
	--build-arg IMAGE_NAME=actyx/cosmos \
	--build-arg GIT_COMMIT=$(git_hash) \
	--build-arg GIT_BRANCH=$(GIT_BRANCH) \
	--build-arg BUILD_RUST_TOOLCHAIN=$(BUILD_RUST_TOOLCHAIN) \
 	--build-arg BUILD_SCCACHE_VERSION=$(BUILD_SCCACHE_VERSION) \
	--build-arg TARGET=$(TARGET) \
	-f $(build_dir)/Dockerfile .
	echo "Cleaning up $(build_dir)"
	rm -rf $(build_dir)


define build_bins_and_move
	$(eval SCCACHE_REDIS?=$(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache))
	mkdir -p $(1)
	docker run -v `pwd`/rt-master:/src \
	-u builder \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-it docker.pkg.github.com/actyx/cosmos/buildrs:x64-latest \
	cargo build --release --target $(2) --bins
	find ./rt-master/target/$(2)/release/ -maxdepth 1 -type f -executable  \
		-exec cp {} $(1) \;
	echo "Please find your build artifacts in $(1)."
endef

actyxos-bin-win64: debug clean
	$(eval ARCH?=win64)
	$(eval TARGET:=x86_64-pc-windows-gnu)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(call build_bins_and_move,$(OUTPUT),$(TARGET))

actyxos-bin-x64: debug clean
	$(eval ARCH?=x64)
	$(eval TARGET:=x86_64-unknown-linux-gnu)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(call build_bins_and_move,$(OUTPUT),$(TARGET))

actyxos-bin-armv7hf:
	$(eval ARCH?=armv7hf)
	$(eval TARGET:=armv7-unknown-linux-gnueabihf)
	$(eval OUTPUT:=./dist/bin/$(ARCH))
	$(call build_bins_and_move,$(OUTPUT),$(TARGET))

# 32 bit
android-store-lib: debug
	$(eval SCCACHE_REDIS?=$(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache))
	docker run -v `pwd`/rt-master:/src \
	-u builder \
	-e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	-it docker.pkg.github.com/actyx/cosmos/buildrs:x64-latest \
	cargo build -p store-lib --release --target i686-linux-android

# 32 bit
android-app: debug
	mkdir -p ./android-shell-app/app/src/main/jniLibs/x86
	cp ./rt-master/target/i686-linux-android/release/libaxstore.so ./android-shell-app/app/src/main/jniLibs/x86/libaxstore.so
	./android-shell-app/bin/prepare-gradle-build.sh
	pushd android-shell-app; \
	./gradlew clean ktlint build assemble; \
	popd
	echo 'APK: ./android-shell-app/app/build/outputs/apk/release/app-release.apk'

android: debug clean android-store-lib android-app
android-install: debug
	adb uninstall io.actyx.shell
	adb install ./android-shell-app/app/build/outputs/apk/release/app-release.apk

