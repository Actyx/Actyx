# Make all for this file should build every artifact in Cosmos, from the various
# rust binaries to the js packages to the websites(s) and windows and android installers.
#
# Finished artifacts will be in dist.
#
# Prerequisites for using this makefile locally:
#
# - vault credentials should be in the `VAULT_TOKEN` environment variable.
#   E.g. `export VAULT_TOKEN=`vault login -token-only -method aws role=dev-ruediger`
# - nvm should be installed. https://github.com/nvm-sh/nvm#install--update-script
# - docker needs to be installed and configured
# - able to access dockerhub
# - the various docker images used for the build should be up to date
#
# You can use make prepare to update the docker images and install required tools.
SHELL := /bin/bash

architectures = aarch64 x86_64 armv7 arm

all-LINUX := $(foreach arch,$(architectures),$(foreach bin,actyxos-linux ax,linux-$(arch)/$(bin))) linux-x86_64/webview-runner.tgz linux-x86_64/accessory.tgz
all-WINDOWS := $(foreach t,actyxos.exe ax.exe installer webview-runner.zip accessory.zip,windows-x86_64/$t)
all-ANDROID := actyxos.apk

CARGO_TEST_JOBS := 8
CARGO_BUILD_JOBS := 8

# this needs to remain the first so it is the default target
all: all-linux all-android all-windows all-js

all-linux: $(patsubst %,dist/bin/%,$(all-LINUX))

all-android: $(patsubst %,dist/bin/%,$(all-ANDROID))

all-windows: $(patsubst %,dist/bin/%,$(all-WINDOWS))

all-js: \
	dist/js/pond \
	dist/js/os-sdk

# Create a `make-always` target that always has the current timestamp.
# Depending on this ensures that the rule is always executed.
.PHONY: make-always
make-always:
	touch $@

export BUILD_RUST_TOOLCHAIN := 1.48.0

export CARGO_HOME ?= $(HOME)/.cargo

# log in to vault and store the token in an environment variable
# to run this locally, set the VAULT_TOKEN environment variable by running vault login with your dev role.
# e.g. `export VAULT_TOKEN=`vault login -token-only -method aws role=dev-ruediger`
export VAULT_ADDR ?= https://vault.actyx.net
export VAULT_TOKEN ?= $(shell VAULT_ADDR=$(VAULT_ADDR) vault login -token-only -method aws role=ops-travis-ci)

# Helper to try out local builds of Docker images
export IMAGE_VERSION := $(or $(LOCAL_IMAGE_VERSION),latest)

# Debug helpers
print-%:
	@echo $* = $($*)

# delete almost all generated artifacts
# this does not need to be run from CI, since it always starts with a fresh checkout anyway.
# use this locally to ensure a truly fresh build.
clean:
	rm -rf rt-master/target/*
	rm -rf web/downloads.actyx.com/node_modules
	rm -rf web/developer.actyx.com/node_modules
	rm -rf js/pond/node_modules
	rm -rf js/os-sdk/node_modules
	rm -rf jvm/os-android/gradle/build
	rm -rf cpp/webview-runner/dist
	rm -rf dist

# mark things with this dependency to run whenever requested
.PHONY: prepare prepare-js prepare-rs prepare-docker prepare-docker-crosscompile

prepare: prepare-js prepare-rs prepare-docker prepare-docker-crosscompile

prepare-docker:
	# used for windows and android rust builds
	docker pull actyx/util:buildrs-x64-$(IMAGE_VERSION)
	# used for linux rust builds
	docker pull actyx/cosmos:musl-aarch64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-x86_64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-armv7-unknown-linux-musleabihf-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-arm-unknown-linux-musleabi-$(IMAGE_VERSION)

export DOCKER_CLI_EXPERIMENTAL := enabled
LITERAL_SPACE :=
LITERAL_SPACE +=
LITERAL_COMMA := ,
prepare-docker-crosscompile:
	./bin/check-docker-requirements.sh check_docker_version
	./bin/check-docker-requirements.sh enable_multi_arch_support
	for i in `docker buildx ls | awk '{print $$1}'`; do docker buildx rm $$i; done
	docker buildx create --platform $(subst $(LITERAL_SPACE),$(LITERAL_COMMA),$(foreach a,$(architectures),$(dockerPlatform-$(a)))) --use

prepare-rs:
	# install rustup
	curl https://sh.rustup.rs -sSf | sh -s -- -y
	rustup install $(BUILD_RUST_TOOLCHAIN)

prepare-js:
	# install nvm
	curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.36.0/install.sh | bash

# create validation targets for all folder inside `./rust`
rust-validation = $(shell arr=(`ls -1 rust`); printf "validate-rust-%s " "$${arr[@]}")
.PHONY: validate-rust $(rust-validation)
validate-rust: $(rust-validation) validate-os

# execute linter, style checker and tests for everything
validate: validate-os validate-rust validate-os-android validate-js validate-website validate-misc

# declare all the validate targets to be phony
.PHONY: validate-os validate-rust-sdk validate-rust-sdk-macros validate-os-android validate-js validate-website validate-misc

CARGO := cargo +$(BUILD_RUST_TOOLCHAIN)

.PHONY: diagnostics

diagnostics:
	@echo HOME = $(HOME)
	@echo USER = $(shell whoami)
	@echo PATH = $(PATH)
	@echo PWD = $(shell pwd)

define mkRustTestRule=
$(TARGET_NAME): cargo-init make-always
  $(eval TARGET_PATH:=rust/$(word 3, $(subst -, ,$(TARGET_NAME))))
	cd $(TARGET_PATH) && $(CARGO) fmt --all -- --check
	cd $(TARGET_PATH) && $(CARGO) --locked clippy --all-targets -- -D warnings
	cd $(TARGET_PATH) && $(CARGO) test --all-features -j $(CARGO_TEST_JOBS)
endef

$(foreach TARGET_NAME,$(rust-validation),$(eval $(mkRustTestRule)))

.PHONY: validate-os
# execute fmt check, clippy and tests for rt-master
validate-os: diagnostics
	cd rt-master && $(CARGO) fmt --all -- --check
	cd rt-master && $(CARGO) --locked clippy -- -D warnings
	cd rt-master && $(CARGO) --locked clippy --tests -- -D warnings
	cd rt-master && $(CARGO) test --all-features -j $(CARGO_TEST_JOBS)

.PHONY: validate-os-android
# execute linter for os-android
validate-os-android: diagnostics
	jvm/os-android/bin/get-keystore.sh
	cd jvm/os-android/ && ./gradlew clean ktlintCheck

# validate all js
validate-js: diagnostics validate-js-pond validate-js-sdk

# validate js pond
validate-js-pond:
	cd js/pond && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run test && \
		npm run build:prod

# validate js sdk
validate-js-sdk:
	cd js/os-sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run test && \
		npm run build

# make js pond
# this is running directly on the host container, so it needs to have nvm installed
dist/js/pond:
	mkdir -p $@
	cd js/pond && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run build:prod && \
		mv `npm pack` ../../$@/

# make js sdk
# this is running directly on the host container, so it needs to have nvm installed
dist/js/os-sdk:
	mkdir -p $@
	cd js/os-sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run build && \
		npm pack && \
		mv actyx-os-sdk-*.tgz ../../$@/

# validate all websites
validate-website: diagnostics validate-website-developer validate-website-downloads

# validate developer.actyx.com
validate-website-developer:
	cd web/developer.actyx.com && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run test

# validate downloads.actyx.com
validate-website-downloads:
	cd web/downloads.actyx.com && source ~/.nvm/nvm.sh && nvm install && \
		npm install

validate-misc: validate-actyxos-node-manager validate-actyxos-win-installer

# run npm install. There don't seem to be any tests.
validate-actyxos-node-manager:
	docker run \
	  -u $(shell id -u) \
	  -v `pwd`:/src \
	  -w /src/misc/actyxos-node-manager \
	  --rm actyx/util:windowsinstallercreator-x64-latest \
	  bash -c "npm install"

validate-actyxos-win-installer: validate-actyxos-node-manager

# combines all the .so files to build actyxos on android
android-libaxosnodeffi: \
	jvm/os-android/app/src/main/jniLibs/x86/libaxosnodeffi.so \
	jvm/os-android/app/src/main/jniLibs/arm64-v8a/libaxosnodeffi.so \
	jvm/os-android/app/src/main/jniLibs/armeabi-v7a/libaxosnodeffi.so

jvm/os-android/app/src/main/jniLibs/x86/libaxosnodeffi.so: rt-master/target/i686-linux-android/release/libaxosnodeffi.so
	mkdir -p $(dir $@)
	cp $< $@

jvm/os-android/app/src/main/jniLibs/arm64-v8a/libaxosnodeffi.so: rt-master/target/aarch64-linux-android/release/libaxosnodeffi.so
	mkdir -p $(dir $@)
	cp $< $@

jvm/os-android/app/src/main/jniLibs/armeabi-v7a/libaxosnodeffi.so: rt-master/target/armv7-linux-androideabi/release/libaxosnodeffi.so
	mkdir -p $(dir $@)
	cp $< $@

# define mapping from os-arch to target
target-linux-aarch64 = aarch64-unknown-linux-musl
target-linux-x86_64 = x86_64-unknown-linux-musl
target-linux-armv7 = armv7-unknown-linux-musleabihf
target-linux-arm = arm-unknown-linux-musleabi
target-windows-x86_64 = x86_64-pc-windows-gnu
# non-musl targets
target-nonmusl-linux-aarch64 = aarch64-unknown-linux-gnu
target-nonmusl-linux-x86_64 = x86_64-unknown-linux-gnu
target-nonmusl-linux-armv7 = armv7-unknown-linux-gnueabihf
target-nonmusl-linux-arm = arm-unknown-linux-gnueabi
target-nonmusl-windows-x86_64 = x86_64-pc-windows-gnu

# define mapping from os to builder image name
image-linux = actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION)
image-windows = actyx/util:buildrs-x64-$(IMAGE_VERSION)

# list all os-arch and binary names
osArch = $(foreach a,$(architectures),linux-$(a)) windows-x86_64
binaries = ax ax.exe actyxos-linux actyxos.exe

# compute list of all OSs (e.g. linux, windows) and rust targets (looking into the target-* vars)
os = $(sort $(foreach oa,$(osArch),$(word 1,$(subst -, ,$(oa)))))
targets = $(sort $(foreach oa,$(osArch),$(target-$(oa))))
targets-nonmusl = $(sort $(foreach oa,$(osArch),$(target-nonmusl-$(oa))))

# build rules for binaries on the current platform (i.e. no cross-building), like ax.exe
# two-step process:
#   - declare dependency from dist/bin/* to the right file in rt-master/target/...
#   - declare how to build the file in rt-master/target/...
dist/bin/current/%: rt-master/target/release/%
	mkdir -p $(dir $@)
	cp -a $< $@
# here % (and thus $*) matches something like ax.exe, so we need to strip the suffix with `basename`
rt-master/target/release/%: make-always
	cd rt-master && cargo --locked build --release --bin $(basename $*)

# In the following the same two-step process is used as for the current os/arch above.
# The difference is that %-patterns won’t works since there are two variables to fill:
# the os-arch string and the binary name. Therefore, we generate all rules by multiplying
# the list of os-arch strings with the possible binaries and using `eval` to synthesize
# one rule for each such combination.
# mkDistRule is the template that is then instantiated by the nested `foreach` below,
# where $(1) and $(2) will be replaced by the loop values for os-arch and binary name, respectively.
define mkDistRule =
dist/bin/$(1)/$(2): rt-master/target/$(target-$(1))/release/$(2)
	mkdir -p $$(dir $$@)
	cp -a $$< $$@
endef
$(foreach oa,$(osArch),$(foreach bin,$(binaries),$(eval $(call mkDistRule,$(oa),$(bin)))))
$(foreach a,$(architectures),$(foreach bin,docker-logging-plugin docker-axosnode,$(eval $(call mkDistRule,linux-$(a),$(bin)))))

# Make a list of pattern rules (with %) for all possible rust binaries
# containing e.g. rt-master/target/aarch64-unknown-linux-musl/release/%.
# These will be used below to define how to build all binaries for that target.
targetPatterns = $(foreach t,$(targets),rt-master/target/$(t)/release/%)

# loopup table for extra cargo options for the windows build.
# for the windows build, we need to default features. This requires specifying a manifest path.
cargo-extra-options-rt-master/target/x86_64-pc-windows-gnu/release/ax.exe:=--no-default-features --manifest-path actyx-cli/Cargo.toml
cargo-extra-options-rt-master/target/x86_64-pc-windows-gnu/release/actyxos.exe:=--no-default-features --manifest-path ax-os-node-win/Cargo.toml
cargo-extra-options-rt-master/target/x86_64-pc-windows-gnu/release/win.exe:=--no-default-features --manifest-path ax-os-node/Cargo.toml

# define a pattern rule for making any binary for a given target
# where the build image is computed by first extracting the OS from the target string and then
# looking into the image-* mapping - this requires the TARGET variable to be set while evaluating!
define mkBinaryRule =
rt-master/target/$(TARGET)/release/%: cargo-init make-always
	docker run \
	  -u $(shell id -u) \
	  -w /src/rt-master \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=$(CARGO_BUILD_JOBS) \
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  $(image-$(word 3,$(subst -, ,$(TARGET)))) \
	  cargo --locked build --release $$(cargo-extra-options-$$@) --bin $$(basename $$*)
endef
$(foreach TARGET,$(targets),$(eval $(mkBinaryRule)))

# targets for which we need a .so file for android
android_so_targets = i686-linux-android aarch64-linux-android armv7-linux-androideabi

# make a list of pattern rules (with %) for all possible .so files needed for android
soTargetPatterns = $(foreach t,$(android_so_targets),rt-master/target/$(t)/release/libaxosnodeffi.so)

# same principle as above for targetPatterns
$(soTargetPatterns): TARGET = $(word 3,$(subst /, ,$@))
$(soTargetPatterns): OS = $(word 3,$(subst -, ,$(TARGET)))
$(soTargetPatterns): cargo-init make-always
	docker run \
	  -u $(shell id -u) \
	  -w /src/rt-master \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=$(CARGO_BUILD_JOBS) \
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
	  cargo --locked build -p ax-os-node-ffi --lib --release --target $(TARGET)

# create these so that they belong to the current user (Docker would create as root)
# (formulating as rule dependencies only runs mkdir when they are missing)
cargo-init: $(CARGO_HOME)/git $(CARGO_HOME)/registry
$(CARGO_HOME)/%:
	mkdir -p $@

jvm/os-android/app/build/outputs/apk/release/app-release.apk: android-libaxosnodeffi make-always
	jvm/os-android/bin/get-keystore.sh
	docker run \
	  -u $(shell id -u) \
	  -v `pwd`:/src \
	  -w /src/jvm/os-android \
	  --rm \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
      ./gradlew ktlintCheck build assembleRelease androidGitVersion

dist/bin/actyxos.apk: jvm/os-android/app/build/outputs/apk/release/app-release.apk
	mkdir -p $(dir $@)
	cp $< $@

misc/actyxos-node-manager/out/ActyxOS-Node-Manager-win32-x64: dist/bin/windows-x86_64/ax.exe make-always
	mkdir -p misc/actyxos-node-manager/bin/win32
	cp dist/bin/windows-x86_64/ax.exe misc/actyxos-node-manager/bin/win32/
	docker run \
	  -u $(shell id -u) \
	  -v `pwd`:/src \
	  -w /src/misc/actyxos-node-manager \
	  --rm actyx/util:windowsinstallercreator-x64-latest \
	  bash -c "npm install && npm run package -- --platform win32 --arch x64"

dist/bin/windows-x86_64/actyxos-node-manager.exe: misc/actyxos-node-manager/out/ActyxOS-Node-Manager-win32-x64
	mkdir -p $(dir $@)
	cp -a $</actyxos-node-manager.exe $@

dist/bin/windows-x86_64/installer: misc/actyxos-node-manager/out/ActyxOS-Node-Manager-win32-x64 dist/bin/windows-x86_64/ax.exe dist/bin/windows-x86_64/actyxos.exe make-always
	mkdir -p $@
	cp $</actyxos-node-manager.exe misc/actyxos-win-installer
	cp dist/bin/windows-x86_64/actyxos.exe misc/actyxos-win-installer
	cp dist/bin/windows-x86_64/ax.exe misc/actyxos-win-installer
	cp -r misc/actyxos-node-manager/out/ActyxOS-Node-Manager-win32-x64 misc/actyxos-win-installer/node-manager
	# ls -alh .
	docker run \
	  -u $(shell id -u) \
	  -v `pwd`:/src \
	  -w /src/misc/actyxos-win-installer \
	  -e DIST_DIR='/src/dist/bin/windows-x86_64/installer' \
	  -e SRC_DIR='/src/misc/actyxos-win-installer' \
	  -e PRODUCT_VERSION=1.0.1-SNAPSHOT \
	  -e PRODUCT_NAME=ActyxOS \
	  -e INSTALLER_NAME='ActyxOS Installer' \
	  --rm \
	  actyx/util:windowsinstallercreator-x64-latest \
	  ./build.sh


# Getting the current git branch and commit hash
gitBranch=$(shell echo `git rev-parse --abbrev-ref HEAD` | sed -e "s,refs/heads/,,g")
gitHash=$(shell git log -1 --pretty=%H)

# For each directory inside ${dockerDir}, create targets for each architecture, e.g.
# 	actyxos --> docker-build-actyxos-x86_64
# 			--> docker-build-actyxos-aarch64
# 			--> docker-build-actyxos-arm
# 			--> docker-build-actyxos-armv7
#
# The `docker buildx` command will be used to cross compile the images.
dockerDir = ops/docker/images
dockerTargetPatterns = $(foreach a,$(architectures),$(shell arr=(`ls -1 ${dockerDir} | grep -v -e "^musl\\$$"`); printf "docker-build-%s-$(a)\n " "$${arr[@]}"))
dockerBuildDir = dist/docker

# mapping from arch to docker platform
dockerPlatform-aarch64 = linux/arm64
dockerPlatform-x86_64 = linux/amd64
dockerPlatform-armv7 = linux/arm/v7
dockerPlatform-arm = linux/arm/v6
DOCKER_REPO ?= actyx/cosmos
getImageNameDockerhub = $(DOCKER_REPO):$(1)-$(2)-$(3)
$(dockerTargetPatterns): make-always
	# must not use `component` here because of dependencies
	$(eval DOCKER_TAG:=$(subst docker-build-,,$@))
	$(eval arch:=$(shell echo $(DOCKER_TAG) | cut -f2 -d-))
	$(eval DOCKER_IMAGE_NAME:=$(shell echo $(DOCKER_TAG) | cut -f1 -d-))
	echo $(DOCKER_TAG) $(arch) $(DOCKER_IMAGE_NAME)
	mkdir -p $(dockerBuildDir)
	cp -RPp $(dockerDir)/$(DOCKER_IMAGE_NAME)/* $(dockerBuildDir)
	$(eval IMAGE_NAME:=$(call getImageNameDockerhub,$(DOCKER_IMAGE_NAME),$(arch),$(gitHash)))
	if [ -f $(dockerBuildDir)/prepare-image.sh ]; then \
	  export ARCH=$(arch); \
	  export GIT_HASH=$(gitHash); \
	  cd $(dockerBuildDir); \
	  echo 'Running prepare script'; \
	  ./prepare-image.sh ..; \
	fi

	DOCKER_CLI_EXPERIMENTAL=enabled DOCKER_BUILDKIT=1 docker buildx build \
	--platform $(dockerPlatform-$(arch)) \
	--load \
	--tag $(IMAGE_NAME) \
	--build-arg BUILD_DIR=$(dockerBuildDir) \
	--build-arg ARCH=$(arch) \
	--build-arg ARCH_AND_GIT_TAG=$(arch)-$(gitHash) \
	--build-arg IMAGE_NAME=actyx/cosmos \
	--build-arg GIT_COMMIT=$(gitHash) \
	--build-arg gitBranch=$(gitBranch) \
	--build-arg BUILD_RUST_TOOLCHAIN=$(BUILD_RUST_TOOLCHAIN) \
	-f $(dockerBuildDir)/Dockerfile .
	echo "Cleaning up $(dockerBuildDir)"
	rm -rf $(dockerBuildDir)

$(foreach a,$(architectures),$(eval docker-build-dockerloggingplugin-$(a): dist/bin/linux-$(a)/docker-logging-plugin))
$(foreach a,$(architectures),$(eval docker-build-actyxos-$(a): dist/bin/linux-$(a)/docker-axosnode docker-build-dockerloggingplugin-$(a)))

docker-build-actyxos: $(foreach a,$(architectures),docker-build-actyxos-$(a))

# Webview-runner
/tmp/cef-%-x86_64:
	mkdir -p $@ && \
	wget -qO- https://cef-builds.spotifycdn.com/cef_binary_86.0.21%2Bg6a2c8e7%2Bchromium-86.0.4240.183_$*64_minimal.tar.bz2 | tar --strip-components 1 -jxf - -C $@

dist/bin/linux-x86_64/webview-runner.tgz: /tmp/cef-linux-x86_64
	$(eval target_dir:=$(dir $@))
	mkdir -p $(target_dir)
	mkdir -p cpp/webview-runner/dist
	docker run \
	  -u $(shell id -u) \
	  -v `pwd`/cpp/webview-runner:/src \
	  -w /src/dist \
	  -e HOME=/home/builder \
	  -v /tmp/cef-linux-x86_64:/tmp/cef \
	  -e CEF_ROOT=/tmp/cef \
	  --rm \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
	  bash -c 'cmake -G "Unix Makefiles" -DCMAKE_BUILD_TYPE=Release .. && make -j$(CARGO_BUILD_JOBS) webview-runner'
	tar cvfz $@ -C cpp/webview-runner/dist/src/Release .

# The docker image `actyx/util:msvc16-x64-latest` is used for cross compilation
# to Windows. This image uses wine to run the MSVC toolchain, as CEF3 doesn't
# support MinGW.  The container has been created with
# https://github.com/Actyx/MSVCDocker
dist/bin/windows-x86_64/webview-runner.zip: /tmp/cef-windows-x86_64
	$(eval target_dir:=$(dir $@))
	mkdir -p $(target_dir)
	cd cpp/webview-runner && \
		rm -rf dist && \
		mkdir dist && \
		docker run \
		  -v `pwd`:/host/ \
			-w /host/dist \
			-u $(shell id -u):$(shell id -g) \
			-e CEF_ROOT=Z:/tmp/cef \
			-v /tmp/cef-windows-x86_64:/tmp/cef \
			-e MSVCARCH=64 \
			--rm \
			actyx/util:msvc16-x64-latest \
			bash -c 'vcwine cmake -G "NMake Makefiles JOM" -DCMAKE_BUILD_TYPE=Release -Wno-dev .. && vcwine jom'
	(cd cpp/webview-runner/dist/src/Release && zip -r - .) > $@
## Webview-runner END

## Accessory
# Only x86_64 so far ..
accessoryPatterns = $(foreach t,$(targets),rust/accessory/target/$(t)/release/%)
rust/accessory/target/release/%: make-always
	cd rust/accessory && cargo --locked build --release

# libudev being LGPL doesn't allow to statically link against, so no musl for us
define mkAccessoryBinaryRule =
rust/accessory/target/$(TARGET)/release/%: cargo-init make-always
	docker run \
	  -u $(shell id -u) \
	  -w /src/rust/accessory \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=$(CARGO_BUILD_JOBS) \
	  -e HOME=/home/builder \
	  -e PKG_CONFIG_ALLOW_CROSS=1 \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
	  cargo --locked build --release
endef
targets-accessory = $(sort $(foreach oa,linux-x86_64 windows-x86_64,$(target-nonmusl-$(oa))))
$(foreach TARGET,$(targets-accessory),$(eval $(mkAccessoryBinaryRule)))

dist/bin/linux-x86_64/accessory: rust/accessory/target/x86_64-unknown-linux-gnu/release/accessory
	mkdir -p $@
	cp $(dir $<)/{accessory,libbrp_lib.so} $@

dist/bin/linux-x86_64/accessory.tgz: dist/bin/linux-x86_64/accessory dist/bin/linux-x86_64/accessory/libbrp_lib.so
	tar cvfz $@ -C $< .

dist/bin/windows-x86_64/accessory: rust/accessory/target/x86_64-pc-windows-gnu/release/accessory.exe
	mkdir -p $@
	cp $(dir $<)/{accessory.exe,libbrp_lib.dll} $@

dist/bin/windows-x86_64/accessory.zip: dist/bin/windows-x86_64/accessory dist/bin/windows-x86_64/accessory/libbrp_lib.dll
	(cd $< && zip -r - .) > $@
## Accessory END
