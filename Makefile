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
build_dir=dist
DOCKER_DIR=ops/docker/images
DOCKER_BUILD = $(shell arr=(`ls ${DOCKER_DIR}`); printf "docker-build-%s\n " "$${arr[@]}")
DOCKER_BUILD_SBT = docker-build-bagsapconnector docker-build-flexishttpconnector docker-build-flexisftpconnector docker-build-opcuaconnector docker-build-iafisconnector docker-build-batchcenterconnector

# Debug helpers
print-%  : ; @echo $* = $($*)
debug: print-GIT_BRANCH print-git_hash print-component print-arch print-build_dir

.PHONY: all ${DOCKER_BUILD} clean

all: clean ${DOCKER_BUILD}

clean:
	rm -rf $(build_dir)

docker-login-dockerhub:
	docker login -u $(DOCKERHUB_USER) -p $(DOCKERHUB_PASS)

docker-login-github:
	docker login docker.pkg.github.com -u $(GITHUB_PKG_USER) -p $(GITHUB_PKG_PASS)

docker-login: docker-login-dockerhub docker-login-github

getImageName = actyx/cosmos:$(1)-$(2)-$(3)
getImageNameGithub = docker.pkg.github.com/actyx/cosmos/$(1):$(2)-$(3)

docker-push-%: docker-build-% docker-login
	$(eval DOCKER_IMAGE_NAME:=$(subst docker-push-,,$@))
	$(eval IMAGE_NAME:=$(call getImageName,$(DOCKER_IMAGE_NAME),$(arch),$(git_hash)))
	$(eval IMAGE_NAME_GH:=$(call getImageNameGithub,$(DOCKER_IMAGE_NAME),$(arch),$(git_hash)))
	docker push $(IMAGE_NAME)
	docker tag $(IMAGE_NAME) $(IMAGE_NAME_GH)
	docker push $(IMAGE_NAME_GH)
	$(eval LATEST_IMAGE_TAG:=$(call getImageName,$(DOCKER_IMAGE_NAME),$(arch),latest))
	$(eval LATEST_IMAGE_TAG_GH:=$(call getImageNameGithub,$(DOCKER_IMAGE_NAME),$(arch),latest))
	if [ $(GIT_BRANCH) == "master" ]; then \
	  docker tag $(IMAGE_NAME) $(LATEST_IMAGE_TAG); \
		docker push $(LATEST_IMAGE_TAG); \
	  docker tag $(IMAGE_NAME) $(LATEST_IMAGE_TAG_GH); \
		docker push $(LATEST_IMAGE_TAG_GH); \
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
	if [ "$(arch)" == 'arm' ]; then \
		cd $(build_dir); \
		echo "arch is $(arch) - generating Dockerfile using gen-$(arch).sh"; \
		mv Dockerfile Dockerfile-x64; \
		../ops/docker/gen-$(arch).sh ./Dockerfile-x64 ./Dockerfile; \
	fi
	if [ -f $(build_dir)/prepare-image.sh ]; then \
		cd $(build_dir); \
		echo 'Running prepare script'; \
		./prepare-image.sh ..; \
	fi
	DOCKER_BUILDKIT=1 docker build -t $(IMAGE_NAME) \
	--build-arg BUILD_DIR=$(build_dir) --build-arg ARCH_AND_GIT_TAG=$(arch)-$(git_hash) --build-arg IMAGE_NAME=actyx/cosmos \
	--build-arg GIT_COMMIT=$(git_hash) --build-arg GIT_BRANCH=$(GIT_BRANCH) --build-arg BUILD_RUST_TOOLCHAIN=$(BUILD_RUST_TOOLCHAIN) \
 	--build-arg BUILD_SCCACHE_VERSION=$(BUILD_SCCACHE_VERSION) \
	-f $(build_dir)/Dockerfile .
	echo 'Cleaning up $(build_dir)'
	rm -rf $(build_dir)


# 32 bit
android-store-lib: debug
	docker run -v `pwd`/rt-master:/root/src -it actyx/cosmos:build-android-rs-x64-latest cargo build -p store-lib --release --target i686-linux-android

# 32 bit
android-app: debug
	mkdir -p ./android-shell-app/app/src/main/jniLibs/x86
	cp ./rt-master/target/i686-linux-android/release/libaxstore.so ./android-shell-app/app/src/main/jniLibs/x86/libaxstore.so
	./android-shell-app/bin/prepare-gradle-build.sh
	pushd android-shell-app; \
	./gradlew clean ktlint build assemble; \
	popd

android: debug android-store-lib android-app


# Docker build dependencies
docker-build-hammerite: docker-build-adaclir

