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
# - if you want to use SCCACHE to speed up rust builds, make sure the SCCACHE_REDIS
#   environment variable is set:
#   export SCCACHE_REDIS=`vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache`
#   the default is to not use it
SHELL := /bin/bash

all-LINUX := $(foreach arch,x86_64 aarch64 armv7 arm,linux-$(arch)/actyxos-linux)
all-WINDOWS := windows-x86_64/actyxos.exe windows-x86_64/ax.exe
all-ANDROID := actyxos.apk

CARGO_TEST_JOBS := 8
CARGO_BUILD_JOBS := 8

# this needs to remain the first so it is the default target
all: all-linux all-android all-windows all-js

all-linux: $(patsubst %,dist/bin/%,$(all-LINUX))

all-android: $(patsubst %,dist/bin/%,$(all-ANDROID))

all-windows: $(patsubst %,dist/bin/%,$(all-WINDOWS)) dist/bin/windows-x86_64/installer

all-js: \
	dist/js/pond \
	dist/js/os-sdk

# These should be moved to the global azure pipelines build
export BUILD_RUST_TOOLCHAIN := 1.45.0
export BUILD_SCCACHE_VERSION := 0.2.12

export CARGO_HOME ?= $(HOME)/.cargo

# log in to vault and store the token in an environment variable
# to run this locally, set the VAULT_TOKEN environment variable by running vault login with your dev role.
# e.g. `export VAULT_TOKEN=`vault login -token-only -method aws role=dev-ruediger`
export VAULT_TOKEN ?= $(shell vault login -token-only -method aws role=ops-travis-ci)

# export SCCACHE_REDIS ?= $(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache)
SCCACHE_REDIS ?= $(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache)

# Helper to try out local builds of Docker images
export IMAGE_VERSION := $(or $(LOCAL_IMAGE_VERSION),latest)

# Debug helpers
print-%:
	@echo $* = $($*)

clean:
	cd rt-master/target && rm -rf *
	cd web/downloads.actyx.com && rm -rf node_modules
	cd web/developer.actyx.com && rm -rf node_modules
	cd js/pond && rm -rf node_modules
	cd js/os-sdk && rm -rf node_modules
	cd jvm/os-android/gradle && rm -rf build
	cd dist && rm -rf *

# mark things with this dependency to run whenever requested
.PHONY: prepare prepare-js prepare-rs

prepare: prepare-js prepare-rs
	# used for windows and android rust builds
	docker pull actyx/util:buildrs-x64-$(IMAGE_VERSION)
	# used for linux rust builds
	docker pull actyx/cosmos:musl-aarch64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-x86_64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-armv7-unknown-linux-musleabihf-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-arm-unknown-linux-musleabi-$(IMAGE_VERSION)

prepare-rs:
	# install rustup
	curl https://sh.rustup.rs -sSf | sh -s -- -y
	rustup install $(BUILD_RUST_TOOLCHAIN)

prepare-js:
	# install nvm
	curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.36.0/install.sh | bash

# execute linter, style checker and tests for everything
validate: validate-os validate-rust-sdk validate-rust-sdk-macros validate-os-android validate-js validate-website validate-misc

# declare all the validate targets to be phony
.PHONY: validate-os validate-rust-sdk validate-rust-sdk-macros validate-os-android validate-js validate-website validate-misc

CARGO := cargo +$(BUILD_RUST_TOOLCHAIN)

.PHONY: diagnostics

diagnostics:
	@echo HOME = $(HOME)
	@echo USER = $(shell whoami)
	@echo PATH = $(PATH)
	@echo PWD = $(shell pwd)

.PHONY: validate-os
# execute fmt check, clippy and tests for rt-master
validate-os: diagnostics
	cd rt-master && $(CARGO) fmt --all -- --check
	cd rt-master && $(CARGO) --locked clippy -- -D warnings
	cd rt-master && $(CARGO) --locked clippy --tests -- -D warnings
	cd rt-master && $(CARGO) test --all-features -j $(CARGO_TEST_JOBS)

.PHONY: validate-rust-sdk
# execute fmt check, clippy and tests for rust-sdk
validate-rust-sdk:
	cd rust/sdk && $(CARGO) fmt --all -- --check
	cd rust/sdk && $(CARGO) --locked clippy -- -D warnings
	cd rust/sdk && $(CARGO) --locked clippy --tests -- -D warnings
	cd rust/sdk && $(CARGO) test --all-features -j $(CARGO_TEST_JOBS)

.PHONY: validate-rust-sdk-macros
# execute fmt check, clippy and tests for rust-sdk-macros
validate-rust-sdk-macros:
	cd rust/sdk_macros && $(CARGO) fmt --all -- --check
	cd rust/sdk_macros && $(CARGO) --locked clippy -- -D warnings
	cd rust/sdk_macros && $(CARGO) --locked clippy --tests -- -D warnings
	cd rust/sdk_macros && $(CARGO) test --all-features -j $(CARGO_TEST_JOBS)

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
		npm i && \
		npm run test && \
		npm run build:prod

# validate js sdk
validate-js-sdk:
	cd js/os-sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm i && \
		npm run test && \
		npm run build

# make js pond
# this is running directly on the host container, so it needs to have nvm installed
dist/js/pond:
	mkdir -p $@
	cd js/pond && source ~/.nvm/nvm.sh && nvm install && \
		npm i && \
		npm run build:prod && \
		mv `npm pack` ../../$@/

# make js sdk
# this is running directly on the host container, so it needs to have nvm installed
dist/js/os-sdk:
	mkdir -p $@
	cd js/os-sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm i && \
		npm run build && \
		npm pack && \
		mv actyx-os-sdk-*.tgz ../../$@/

# validate all websites
validate-website: diagnostics validate-website-developer validate-website-downloads

# validate developer.actyx.com
validate-website-developer:
	cd web/developer.actyx.com && source ~/.nvm/nvm.sh && nvm install && \
		npm i && \
		npm run test

# validate downloads.actyx.com
validate-website-downloads:
	cd web/downloads.actyx.com && source ~/.nvm/nvm.sh && nvm install && \
		npm i

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

# define mapping from os to builder image name
image-linux = actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION)
image-windows = actyx/util:buildrs-x64-$(IMAGE_VERSION)

# list all os-arch and binary names
osArch = linux-aarch64 linux-x86_64 linux-armv7 linux-arm windows-x86_64
binaries = ax ax.exe actyxos-linux actyxos.exe

# compute list of all OSs (e.g. linux, windows) and rust targets (looking into the target-* vars)
os = $(sort $(foreach oa,$(osArch),$(word 1,$(subst -, ,$(oa)))))
targets = $(sort $(foreach oa,$(osArch),$(target-$(oa))))

# build rules for binaries on the current platform (i.e. no cross-building), like ax.exe
# two-step process:
#   - declare dependency from dist/bin/* to the right file in rt-master/target/...
#   - declare how to build the file in rt-master/target/...
dist/bin/current/%: rt-master/target/release/%
	mkdir -p $(dir $@)
	cp $< $@
# here % (and thus $*) matches something like ax.exe, so we need to strip the suffix with `basename`
rt-master/target/release/%:
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
	cp $$< $$@
endef
$(foreach oa,$(osArch),$(foreach bin,$(binaries),$(eval $(call mkDistRule,$(oa),$(bin)))))

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
rt-master/target/$(TARGET)/release/%: cargo-init
	docker run \
	  -u $(shell id -u) \
	  -w /src/rt-master \
	  -e SCCACHE_REDIS=$(SCCACHE_REDIS) \
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
$(soTargetPatterns): cargo-init
	docker run \
	  -u $(shell id -u) \
	  -w /src/rt-master \
	  -e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=$(CARGO_BUILD_JOBS) \
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  actyx/util:buildrs-x64-latest \
	  cargo --locked build -p ax-os-node-ffi --lib --release --target $(TARGET)

# create these so that they belong to the current user (Docker would create as root)
# (formulating as rule dependencies only runs mkdir when they are missing)
cargo-init: $(CARGO_HOME)/git $(CARGO_HOME)/registry
$(CARGO_HOME)/%:
	mkdir -p $@

jvm/os-android/app/build/outputs/apk/release/app-release.apk: android-libaxosnodeffi
	jvm/os-android/bin/get-keystore.sh
	docker run \
	  -u $(shell id -u) \
	  -v `pwd`:/src \
	  -w /src/jvm/os-android \
	  --rm \
	  actyx/util:buildrs-x64-latest \
      ./gradlew ktlintCheck build assembleRelease androidGitVersion

dist/bin/actyxos.apk: jvm/os-android/app/build/outputs/apk/release/app-release.apk
	mkdir -p $(dir $@)
	cp $< $@

misc/actyxos-node-manager/out/ActyxOS-Node-Manager-win32-x64: dist/bin/windows-x86_64/ax.exe
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
	cp $</actyxos-node-manager.exe $@

dist/bin/windows-x86_64/installer: misc/actyxos-node-manager/out/ActyxOS-Node-Manager-win32-x64 dist/bin/windows-x86_64/ax.exe dist/bin/windows-x86_64/actyxos.exe
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
