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
#
# Useful make targets (<arch> should be replaced by one of the values in the `architectures`
# variable):
#   Prepare your environment:
#     prepare
#
#   Validate code (unit tests):
#     validate
#     validate-{actyx-win-installer,js,js-pond,js-sdk,misc,os,os-android,website,website-developer,website-downloads}
#
#   Generate artifacts (stored in dist/):
#     all (default target)
#     all-{windows,android,macos,linux,js}
#     linux-<arch>
#
#   Build Actyx Docker images
#     docker-<arch>
#     docker-all
#     docker-multiarch
#     docker-current
#
# Useful environment variable overrides:
#   CARGO_TEST_JOBS (default 8) will set the number of threads that cargo will use for testing
#   CARGO_BUILD_JOBS (default 8) will set the number of threads that cargo will use for compiling
#   BUILD_RUST_TOOLCHAIN set to test building with a different toolchain than the default
#   LOCAL_IMAGE_VERSION set to change the Git commit to be used for the musl and buildrs images

SHELL := /bin/bash

MIN_MAKE_VERSION := 4.2
# This checks the make version and aborts with an error if it's not at least MIN_MAKE_VERSION
ok := $(filter $(MIN_MAKE_VERSION),$(firstword $(sort $(MAKE_VERSION) $(MIN_MAKE_VERSION))))
ifndef ok
$(error Please upgrade to GNU Make $(MIN_MAKE_VERSION) you are on: $(MAKE_VERSION))
endif

#############################
##### Configuration variables
#############################
architectures = aarch64 x86_64 armv7 arm
unix-bins = actyx-linux ax
windows-bins = actyx.exe ax.exe Actyx-Installer.exe
android-bins = actyx.apk

CARGO_TEST_JOBS ?= 8
CARGO_BUILD_JOBS ?= 8

export BUILD_RUST_TOOLCHAIN ?= 1.51.0

# The stable image version is the git commit hash inside `Actyx/Cosmos`, with
# which the respective images was built. Whenever the build images (inside
# ops/docker/images/{buildrs,musl}/Dockerfile) are modified (meaning built and
# pushed), this needs to be changed.
export LATEST_STABLE_IMAGE_VERSION := 91d2744dfb87621c93940e32b1f183897eeec967

# Mapping from os-arch to target
target-linux-aarch64 = aarch64-unknown-linux-musl
target-linux-x86_64 = x86_64-unknown-linux-musl
target-linux-armv7 = armv7-unknown-linux-musleabihf
target-linux-arm = arm-unknown-linux-musleabi
target-windows-x86_64 = x86_64-pc-windows-gnu
target-macos-x86_64 = x86_64-apple-darwin
target-macos-aarch64 = aarch64-apple-darwin

# non-musl targets
target-nonmusl-linux-aarch64 = aarch64-unknown-linux-gnu
target-nonmusl-linux-x86_64 = x86_64-unknown-linux-gnu
target-nonmusl-linux-armv7 = armv7-unknown-linux-gnueabihf
target-nonmusl-linux-arm = arm-unknown-linux-gnueabi
target-nonmusl-windows-x86_64 = x86_64-pc-windows-gnu

# Mapping from arch to Docker buildx platform
docker-platform-x86_64 = linux/amd64
docker-platform-aarch64 = linux/arm64/v8
docker-platform-armv7 = linux/arm/v7
docker-platform-arm = linux/arm/v6

# Mapping from os to builder image name
image-linux = actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION)
image-windows = actyx/util:buildrs-x64-$(IMAGE_VERSION)
# see https://github.com/Actyx/osxbuilder
image-darwin = actyx/osxbuilder:90af262c037444c4da6d981f8a885ac510a79bb6

# list all os-arch and binary names
osArch = $(foreach a,$(architectures),linux-$(a)) windows-x86_64 macos-x86_64 macos-aarch64
binaries = ax ax.exe actyx-linux actyx.exe

# targets for which we need a .so file for android
android_so_targets = i686-linux-android aarch64-linux-android armv7-linux-androideabi

CARGO := RUST_BACKTRACE=1  cargo +$(BUILD_RUST_TOOLCHAIN)

#################################
##### END Configuration variables
#################################

export GIT_COMMIT = $(shell git rev-parse --short HEAD)$(shell [ -n "$(shell git status --porcelain)" ] && echo _dirty)
export ACTYX_VERSION ?= 0.0.0_dev-$(GIT_COMMIT)
export ACTYX_VERSION_CLI ?= 0.0.0_dev-$(GIT_COMMIT)
export ACTYX_VERSION_NODE-MANAGER ?= 0.0.0_dev-$(GIT_COMMIT)

all-WINDOWS := $(foreach t,$(windows-bins),windows-x86_64/$t)
all-ANDROID := $(android-bins)
all-MACOS := $(foreach t,$(unix-bins),macos-x86_64/$t macos-aarch64/$t)

docker-platforms = $(foreach arch,$(architectures),$(docker-platform-$(arch)))
docker-build-args = --build-arg ACTYX_VERSION=$(ACTYX_VERSION) --build-arg GIT_COMMIT=$(GIT_COMMIT)
docker-multiarch-build-args = $(docker-build-args) --platform $(shell echo $(docker-platforms) | sed 's/ /,/g')

export CARGO_HOME ?= $(HOME)/.cargo
export DOCKER_CLI_EXPERIMENTAL := enabled

# log in to vault and store the token in an environment variable
# to run this locally, set the VAULT_TOKEN environment variable by running vault login with your dev role.
# e.g. `export VAULT_TOKEN=`vault login -token-only -method aws role=dev-ruediger`
# the current token is looked up and no login attempted if present - this suppresses warnings
VAULT_TOKEN ?= $(vault token lookup -format=json | jq .data.id)
ifndef VAULT_TOKEN
export VAULT_ADDR ?= https://vault.actyx.net
export VAULT_TOKEN ?= $(shell VAULT_ADDR=$(VAULT_ADDR) vault login -token-only -method aws role=ops-travis-ci)
endif

# Use docker run -ti only if the input device is a TTY (so that Ctrl+C works)
export DOCKER_FLAGS ?= -e "ACTYX_VERSION=${ACTYX_VERSION}" -e "ACTYX_VERSION_CLI=${ACTYX_VERSION_CLI}" $(shell if test -t 0; then echo "-ti"; else echo ""; fi)

# Helper to try out local builds of Docker images
export IMAGE_VERSION := $(or $(LOCAL_IMAGE_VERSION),$(LATEST_STABLE_IMAGE_VERSION))

# this needs to remain the first so it is the default target
all: all-linux all-android all-windows all-macos all-js

all-android: $(patsubst %,dist/bin/%,$(all-ANDROID))

all-windows: $(patsubst %,dist/bin/%,$(all-WINDOWS))

all-macos: $(patsubst %,dist/bin/%,$(all-MACOS))

all-linux: $(foreach arch,$(architectures),linux-$(arch))

define mkLinuxRule =
linux-$(1): $(foreach bin,$(unix-bins),dist/bin/linux-$(1)/$(bin))
endef

$(foreach arch,$(architectures),$(eval $(call mkLinuxRule,$(arch))))

current: dist/bin/current/ax dist/bin/current/actyx-linux

all-js: \
	dist/js/sdk \
	dist/js/pond

# Create a `make-always` target that always has the current timestamp.
# Depending on this ensures that the rule is always executed.
.PHONY: make-always
make-always:
	touch $@

# Debug helpers
print-%:
	@echo $* = $($*)

# delete almost all generated artifacts
# this does not need to be run from CI, since it always starts with a fresh checkout anyway.
# use this locally to ensure a truly fresh build.
clean:
	rm -rf rust/actyx/target/*
	rm -rf web/downloads.actyx.com/node_modules
	rm -rf web/developer.actyx.com/node_modules
	rm -rf js/sdk/node_modules
	rm -rf js/pond/node_modules
	rm -rf jvm/os-android/gradle/build
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
	docker pull actyx/util:node-manager-win-builder

prepare-docker-crosscompile:
	./bin/check-docker-requirements.sh check_docker_version
	./bin/check-docker-requirements.sh enable_multi_arch_support
	for i in `docker buildx ls | awk '{print $$1}'`; do docker buildx rm $$i; done
	docker buildx create --use

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
	cd $(TARGET_PATH) && $(CARGO) test --locked --all-features -j $(CARGO_TEST_JOBS)
endef

$(foreach TARGET_NAME,$(rust-validation),$(eval $(mkRustTestRule)))

.PHONY: validate-os
# execute fmt check, clippy and tests for rust/actyx
validate-os: diagnostics
	cd rust/actyx && $(CARGO) fmt --all -- --check
	cd rust/actyx && $(CARGO) --locked clippy -- -D warnings
	cd rust/actyx && $(CARGO) --locked clippy --tests -- -D warnings
	cd rust/actyx && $(CARGO) --locked test --all-features -j $(CARGO_TEST_JOBS)

validate-netsim: diagnostics
	cd rust/actyx && $(CARGO) build -p swarm-cli -p swarm-harness --release
	rust/actyx/target/release/gossip --n-nodes 10 --enable-fast-path
	rust/actyx/target/release/gossip --n-nodes 10 --enable-slow-path
	rust/actyx/target/release/gossip --n-nodes 10 --enable-root-map
	rust/actyx/target/release/root_map --n-nodes 10 --enable-root-map
	rust/actyx/target/release/discovery --n-bootstrap 1 --enable-root-map
	rust/actyx/target/release/discovery_multi_net
	rust/actyx/target/release/discovery_external
	rust/actyx/target/release/subscribe --n-nodes 10
	rust/actyx/target/release/query --n-nodes 10
	rust/actyx/target/release/quickcheck_subscribe
	rust/actyx/target/release/quickcheck_interleaved
	rust/actyx/target/release/quickcheck_stress_single_store
	rust/actyx/target/release/quickcheck_ephemeral

.PHONY: validate-os-android
# execute linter for os-android
validate-os-android: diagnostics
	jvm/os-android/bin/get-keystore.sh
	cd jvm/os-android/ && ./gradlew clean ktlintCheck

# validate all js
validate-js: diagnostics validate-js-sdk validate-js-pond

# validate js sdk
validate-js-sdk:
	cd js/sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run test && \
		npm run build

# validate js pond
validate-js-pond:
	cd js/pond && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run test && \
		npm run build:prod

# make js sdk
# this is running directly on the host container, so it needs to have nvm installed
dist/js/sdk: make-always
	mkdir -p $@
	cd js/sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run build:prod && \
		mv `npm pack` ../../$@/

# make js pond
# this is running directly on the host container, so it needs to have nvm installed
dist/js/pond: make-always
	mkdir -p $@
	cd js/pond && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run build:prod && \
		mv `npm pack` ../../$@/

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

# TODO: add tests
validate-node-manager: diagnostics
	cd misc/actyx-node-manager && \
		source ~/.nvm/nvm.sh && \
		nvm install && \
		npm install && \
		npm run build && \
		npm run make

node-manager-win: prepare-docker
	docker run \
	-v `pwd`:/src \
	-w /src/misc/actyx-node-manager \
	--rm \
	actyx/util:node-manager-win-builder \
	bash -c "npm install && npm run build && npm run dist -- --win --x64 && npm run artifacts"

node-manager-mac-linux:
	cd misc/actyx-node-manager && \
		source ~/.nvm/nvm.sh && \
		nvm install && \
		npm install && \
		npm run build && \
		npm run dist && \
		npm run artifacts


# combines all the .so files to build actyxos on android
android-libaxosnodeffi: \
	jvm/os-android/app/src/main/jniLibs/x86/libaxosnodeffi.so \
	jvm/os-android/app/src/main/jniLibs/arm64-v8a/libaxosnodeffi.so \
	jvm/os-android/app/src/main/jniLibs/armeabi-v7a/libaxosnodeffi.so

jvm/os-android/app/src/main/jniLibs/x86/libaxosnodeffi.so: rust/actyx/target/i686-linux-android/release/libaxosnodeffi.so
	mkdir -p $(dir $@)
	cp $< $@

jvm/os-android/app/src/main/jniLibs/arm64-v8a/libaxosnodeffi.so: rust/actyx/target/aarch64-linux-android/release/libaxosnodeffi.so
	mkdir -p $(dir $@)
	cp $< $@

jvm/os-android/app/src/main/jniLibs/armeabi-v7a/libaxosnodeffi.so: rust/actyx/target/armv7-linux-androideabi/release/libaxosnodeffi.so
	mkdir -p $(dir $@)
	cp $< $@

# compute list of all OSs (e.g. linux, windows) and rust targets (looking into the target-* vars)
os = $(sort $(foreach oa,$(osArch),$(word 1,$(subst -, ,$(oa)))))
targets = $(sort $(foreach oa,$(osArch),$(target-$(oa))))
targets-nonmusl = $(sort $(foreach oa,$(osArch),$(target-nonmusl-$(oa))))

# build rules for binaries on the current platform (i.e. no cross-building), like ax.exe
# two-step process:
#   - declare dependency from dist/bin/* to the right file in rust/actyx/target/...
#   - declare how to build the file in rust/actyx/target/...
dist/bin/current/%: rust/actyx/target/release/%
	mkdir -p $(dir $@)
	cp -a $< $@
# here % (and thus $*) matches something like ax.exe, so we need to strip the suffix with `basename`
rust/actyx/target/release/%: make-always
	cd rust/actyx && cargo --locked build --release --bin $(basename $*)

# In the following the same two-step process is used as for the current os/arch above.
# The difference is that %-patterns won’t works since there are two variables to fill:
# the os-arch string and the binary name. Therefore, we generate all rules by multiplying
# the list of os-arch strings with the possible binaries and using `eval` to synthesize
# one rule for each such combination.
# mkDistRule is the template that is then instantiated by the nested `foreach` below,
# where $(1) and $(2) will be replaced by the loop values for os-arch and binary name, respectively.
define mkDistRule =
dist/bin/$(1)/$(2): rust/actyx/target/$(target-$(1))/release/$(2)
	mkdir -p $$(dir $$@)
	cp -a $$< $$@
endef
$(foreach oa,$(osArch),$(foreach bin,$(binaries),$(eval $(call mkDistRule,$(oa),$(bin)))))
$(foreach a,$(architectures),$(foreach bin,docker-logging-plugin,$(eval $(call mkDistRule,linux-$(a),$(bin)))))

# Make a list of pattern rules (with %) for all possible rust binaries
# containing e.g. rust/actyx/target/aarch64-unknown-linux-musl/release/%.
# These will be used below to define how to build all binaries for that target.
targetPatterns = $(foreach t,$(targets),rust/actyx/target/$(t)/release/%)

# define a pattern rule for making any binary for a given target
# where the build image is computed by first extracting the OS from the target string and then
# looking into the image-* mapping - this requires the TARGET variable to be set while evaluating!
define mkBinaryRule =
rust/actyx/target/$(TARGET)/release/%: cargo-init make-always
	docker run \
	  -u builder \
	  -w /src/rust/actyx \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=$(CARGO_BUILD_JOBS) \
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  $(DOCKER_FLAGS) \
	  $(image-$(word 3,$(subst -, ,$(TARGET)))) \
	  cargo --locked build --release --bin $$(basename $$*)
endef
$(foreach TARGET,$(targets),$(eval $(mkBinaryRule)))

# make a list of pattern rules (with %) for all possible .so files needed for android
soTargetPatterns = $(foreach t,$(android_so_targets),rust/actyx/target/$(t)/release/libaxosnodeffi.so)

# same principle as above for targetPatterns
$(soTargetPatterns): TARGET = $(word 4,$(subst /, ,$@))
$(soTargetPatterns): OS = $(word 4,$(subst -, ,$(TARGET)))
$(soTargetPatterns): cargo-init make-always
	docker run \
	  -u builder \
	  -w /src/rust/actyx \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=$(CARGO_BUILD_JOBS) \
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  $(DOCKER_FLAGS) \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
	  cargo --locked build -p node-ffi --lib --release --target $(TARGET)

# create these so that they belong to the current user (Docker would create as root)
# (formulating as rule dependencies only runs mkdir when they are missing)
cargo-init: $(CARGO_HOME)/git $(CARGO_HOME)/registry
$(CARGO_HOME)/%:
	mkdir -p $@

jvm/os-android/app/build/outputs/apk/release/app-release.apk: android-libaxosnodeffi make-always
	jvm/os-android/bin/get-keystore.sh
	docker run \
	  -u builder \
	  -v `pwd`:/src \
	  -w /src/jvm/os-android \
	  --rm \
	  $(DOCKER_FLAGS) \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
      ./gradlew --stacktrace ktlintCheck build assembleRelease androidGitVersion

dist/bin/actyx.apk: jvm/os-android/app/build/outputs/apk/release/app-release.apk
	mkdir -p $(dir $@)
	cp $< $@

dist/bin/windows-x86_64/Actyx-Installer.exe: dist/bin/windows-x86_64/ax.exe dist/bin/windows-x86_64/actyx.exe make-always
	cp dist/bin/windows-x86_64/actyx.exe misc/actyx-win-installer
	cp dist/bin/windows-x86_64/ax.exe misc/actyx-win-installer
	# ls -alh .
	docker run \
	  -v `pwd`:/src \
	  -w /src/misc/actyx-win-installer \
	  -e DIST_DIR='/src/dist/bin/windows-x86_64' \
	  -e SRC_DIR='/src/misc/actyx-win-installer' \
	  -e PRODUCT_VERSION='$(ACTYX_VERSION)' \
	  -e PRODUCT_NAME=Actyx \
	  -e INSTALLER_NAME='Actyx-Installer' \
	  --rm \
	  $(DOCKER_FLAGS) \
	  actyx/util:windowsinstallercreator-x64-latest \
	  ./build.sh

define mkDockerRule =
docker-$(1):
	docker buildx build \
	  --platform $(docker-platform-$(1)) \
	  $(docker-build-args) \
	  -f ops/docker/images/actyx/Dockerfile \
	  --tag actyx/cosmos:actyx-$(1)-$(GIT_COMMIT) \
	  --load \
	  .
endef

$(foreach arch,$(architectures),$(eval $(call mkDockerRule,$(arch))))

docker-all: $(foreach arch,$(architectures),docker-$(arch))

# this will build the actyx docker image for all supported architectures. the
# resulting images won't be loaded into the local docker daemon, because that
# is not supported yet by docker, but will just remain in the build cache. One
# can either load a single one of them providing the appropriate `--platform`
# and `--load`, or `--push` them to a remote registry (or use the appropriate
# `make docker-build-actyx-<arch>` target)
docker-multiarch:
	docker buildx build \
	  $(docker-multiarch-build-args) \
	  -f ops/docker/images/actyx/Dockerfile \
	  .

# build for local architecture and load into docker daemon
docker-current:
	docker buildx build --load $(docker-build-args) -f ops/docker/images/actyx/Dockerfile .

# This is here to ensure that we use the same build-args here and in artifacts.yml
docker-multiarch-build-args:
	@echo $(docker-multiarch-build-args)
