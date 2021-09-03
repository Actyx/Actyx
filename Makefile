# Make all for this file should build every artifact in Cosmos, from the various
# rust binaries to the js packages to the websites(s) and windows and android installers.
#
# Finished artifacts will be in dist.
#
# Prerequisites for using this makefile locally:
#
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
#     validate-{actyx-win-installer,js,js-pond,js-sdk,wix,os,os-android,website,website-developer,website-downloads}
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
windows-bins = actyx.exe ax.exe actyx-x64.msi
android-bins = actyx.apk actyx.aab

CARGO_TEST_JOBS ?= 8
CARGO_BUILD_JOBS ?= 8
CARGO_BUILD_ARGS ?= --features migration-v1

export BUILD_RUST_TOOLCHAIN ?= 1.54.0

# The stable image version is the git commit hash inside `Actyx/Actyx`, with
# which the respective images was built. Whenever the build images (inside
# docker/{buildrs,musl}/Dockerfile) are modified (meaning built and
# pushed), this needs to be changed.
export LATEST_STABLE_IMAGE_VERSION := 0c6ed1ec7665d45d73bdcb974993175e4676542c

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
image-linux = actyx/util:musl-$(TARGET)-$(IMAGE_VERSION)
image-windows = actyx/util:buildrs-x64-$(IMAGE_VERSION)
# see https://github.com/Actyx/osxbuilder
image-darwin = actyx/osxbuilder:a042cc707998b83704f3cf5d3f0ededc7143d1c3

image-dotnet = mcr.microsoft.com/dotnet/sdk:3.1

# list all os-arch and binary names
osArch = $(foreach a,$(architectures),linux-$(a)) windows-x86_64 macos-x86_64 macos-aarch64
binaries = ax ax.exe actyx-linux actyx.exe

# targets for which we need a .so file for android
android_so_targets = x86_64-linux-android i686-linux-android aarch64-linux-android armv7-linux-androideabi

CARGO := RUST_BACKTRACE=1  cargo +$(BUILD_RUST_TOOLCHAIN)

#################################
##### END Configuration variables
#################################

export GIT_COMMIT = $(shell git rev-parse HEAD)$(shell [ -n "$(shell git status --porcelain)" ] && echo _dirty)
export ACTYX_VERSION ?= 0.0.0_dev-$(GIT_COMMIT)
export ACTYX_VERSION_CLI ?= 0.0.0_dev-$(GIT_COMMIT)
export ACTYX_VERSION_NODEMANAGER ?= 0.0.0-dev-$(GIT_COMMIT)

all-WINDOWS := $(foreach t,$(windows-bins),windows-x86_64/$t)
all-ANDROID := $(android-bins)
all-MACOS := $(foreach t,$(unix-bins),macos-x86_64/$t macos-aarch64/$t)

docker-platforms = $(foreach arch,$(architectures),$(docker-platform-$(arch)))
docker-build-args = --build-arg ACTYX_VERSION=$(ACTYX_VERSION) --build-arg GIT_COMMIT=$(GIT_COMMIT) --build-arg CARGO_BUILD_ARGS="$(CARGO_BUILD_ARGS)"
docker-multiarch-build-args = $(docker-build-args) --platform $(shell echo $(docker-platforms) | sed 's/ /,/g')

export CARGO_HOME ?= $(HOME)/.cargo
export DOCKER_CLI_EXPERIMENTAL := enabled

# Use docker run -ti only if the input device is a TTY (so that Ctrl+C works)
export DOCKER_FLAGS ?= -e "ACTYX_VERSION=${ACTYX_VERSION}" -e "ACTYX_VERSION_CLI=${ACTYX_VERSION_CLI}" $(shell if test -t 0; then echo "-ti"; else echo ""; fi)

# Helper to try out local builds of Docker images
export IMAGE_VERSION := $(or $(LOCAL_IMAGE_VERSION),$(LATEST_STABLE_IMAGE_VERSION))

# this needs to remain the first so it is the default target
# THIS TARGET IS NOT RUN FOR ARTIFACTS — see azure-piplines
all: all-linux all-android all-windows all-macos all-js assert-clean

all-android: $(patsubst %,dist/bin/%,$(all-ANDROID))

all-windows: $(patsubst %,dist/bin/%,$(all-WINDOWS))

all-macos: $(patsubst %,dist/bin/%,$(all-MACOS))

all-linux: $(foreach arch,$(architectures),linux-$(arch))

define mkLinuxRule =
linux-$(1): $(foreach bin,$(unix-bins),dist/bin/linux-$(1)/$(bin))
endef

$(foreach arch,$(architectures),$(eval $(call mkLinuxRule,$(arch))))

current: dist/bin/current/ax dist/bin/current/actyx-linux

all-js: dist/js/pond dist/js/sdk

all-dotnet: dist/dotnet/cli dist/dotnet/sdk-integration

# Create a `make-always` target that always has the current timestamp.
# Depending on this ensures that the rule is always executed.
.PHONY: make-always
make-always:
	touch $@

# Debug helpers
print-%:
	@echo $* = $($*)

.PHONY: assert-clean
assert-clean:
	@if [ -n "$(shell git status --porcelain)" ]; then \
		git status --porcelain; echo "Git directory not clean, exiting"; exit 3; \
	else echo "Git directory is clean";  fi

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
	docker pull actyx/util:musl-aarch64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/util:musl-x86_64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/util:musl-armv7-unknown-linux-musleabihf-$(IMAGE_VERSION)
	docker pull actyx/util:musl-arm-unknown-linux-musleabi-$(IMAGE_VERSION)
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

# execute linter, style checker and tests for everything
# THIS TARGET IS NOT RUN FOR PR VALIDATION — see azure-piplines
validate: validate-rust validate-os validate-netsim validate-release validate-os-android validate-js validate-dotnet assert-clean

# declare all the validate targets to be phony
.PHONY: validate-os validate-os-android validate-js validate-dotnet

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
	cd $(TARGET_PATH) && $(CARGO) --locked clippy -j $(CARGO_BUILD_JOBS) --all-targets -- -D warnings
	cd $(TARGET_PATH) && $(CARGO) test --locked --all-features -j $(CARGO_TEST_JOBS)
endef

$(foreach TARGET_NAME,$(rust-validation),$(eval $(mkRustTestRule)))

.PHONY: validate-os
# execute fmt check, clippy and tests for rust/actyx
validate-os: diagnostics
	cd rust/actyx && $(CARGO) fmt --all -- --check
	cd rust/actyx && $(CARGO) --locked clippy -j $(CARGO_BUILD_JOBS) -- -D warnings
	cd rust/actyx && $(CARGO) --locked clippy -j $(CARGO_BUILD_JOBS) --tests -- -D warnings
	cd rust/actyx && $(CARGO) --locked test --all-features -j $(CARGO_TEST_JOBS)

.PHONY: validate-rust
# execute fmt check, clippy and tests for rust/actyx
validate-rust: diagnostics
	cd rust/sdk && $(CARGO) fmt --all -- --check
	cd rust/sdk && $(CARGO) --locked clippy -j $(CARGO_BUILD_JOBS) -- -D warnings
	cd rust/sdk && $(CARGO) --locked clippy -j $(CARGO_BUILD_JOBS) --tests -- -D warnings
	cd rust/sdk && $(CARGO) --locked test --all-features -j $(CARGO_TEST_JOBS)

.PHONY: validate-release
# execute fmt check, clippy and tests for rust/actyx
validate-release: diagnostics
	cd rust/release && $(CARGO) fmt --all -- --check
	cd rust/release && $(CARGO) --locked clippy -j $(CARGO_BUILD_JOBS) -- -D warnings
	cd rust/release && $(CARGO) --locked clippy -j $(CARGO_BUILD_JOBS) --tests -- -D warnings
	cd rust/release && $(CARGO) --locked test --all-features -j $(CARGO_TEST_JOBS)

validate-netsim: diagnostics
	cd rust/actyx && $(CARGO) build -p swarm-cli -p swarm-harness --release -j $(CARGO_BUILD_JOBS)
	rust/actyx/target/release/gossip --n-nodes 8 --enable-fast-path
	rust/actyx/target/release/gossip --n-nodes 8 --enable-slow-path
	rust/actyx/target/release/gossip --n-nodes 8 --enable-root-map
	rust/actyx/target/release/gossip_protocol --n-nodes 8
	rust/actyx/target/release/root_map --n-nodes 8 --enable-root-map
	rust/actyx/target/release/discovery --n-bootstrap 1 --enable-root-map
	rust/actyx/target/release/discovery_multi_net
	rust/actyx/target/release/discovery_external
	rust/actyx/target/release/subscribe --n-nodes 8
	rust/actyx/target/release/query --n-nodes 8
	rust/actyx/target/release/quickcheck_subscribe
	rust/actyx/target/release/quickcheck_interleaved
	rust/actyx/target/release/quickcheck_stress_single_store
	rust/actyx/target/release/quickcheck_ephemeral
        # https://github.com/Actyx/Actyx/issues/160
	# rust/actyx/target/release/health
	rust/actyx/target/release/read_only

.PHONY: validate-os-android
# execute linter for os-android
validate-os-android: diagnostics
	docker run \
	  -u builder \
	  -v `pwd`:/src \
	  -w /src/jvm/os-android \
	  --rm \
	  $(DOCKER_FLAGS) \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
	  ./gradlew clean ktlintCheck

# validate all js
validate-js: diagnostics validate-js-sdk validate-js-pond validate-js-integration

# validate js sdk
validate-js-sdk:
	cd js/sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run test && \
		npm run build:prod

# validate js pond
validate-js-pond:
	cd js/pond && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run test && \
		npm run build:prod

# validate js integration suite (does it compile?)
validate-js-integration:
	cd integration && source ~/.nvm/nvm.sh && nvm install && npm install && npm run tsc

# fix and test all js projects
fix-js: diagnostics fix-js-sdk fix-js-pond

fix-js-sdk:
	cd js/sdk && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run lint:fix && \
		npm run test && \
		npm run build && \
		npm run api:accept


fix-js-pond:
	cd js/pond && source ~/.nvm/nvm.sh && nvm install && \
		npm install && \
		npm run lint:fix && \
		npm run test && \
		npm run build && \
		npm run api:accept

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

dist/dotnet/cli: make-always
	mkdir -p $@
	docker run --rm -v `pwd`:/src -w /src/dotnet/Actyx-SDK $(image-dotnet) dotnet publish CLI --output /src/$@/

dist/dotnet/sdk-integration: make-always
	mkdir -p $@
	docker run --rm -v `pwd`:/src -w /src/dotnet/Actyx-SDK $(image-dotnet) dotnet publish Sdk.IntegrationTests --output /src/$@/

validate-dotnet: validate-dotnet-sdk

validate-dotnet-sdk:
	docker run --rm -v `pwd`:/src -w /src/dotnet/Actyx-SDK $(image-dotnet) dotnet test Sdk.Tests

validate-node-manager-bindings:
	cd rust/actyx/node-manager-bindings && \
		source ~/.nvm/nvm.sh && \
		nvm install && \
		npm install && \
		npm run build

node-manager-win:
	docker run \
	-v `pwd`:/src \
	-w /src/js/node-manager \
	--rm \
	actyx/util:node-manager-win-builder-$(IMAGE_VERSION) \
	bash -c "npm install && npm version $(ACTYX_VERSION_NODEMANAGER) && npm run build && npm run dist -- --win --x64 && npm run artifacts"

node-manager-mac-linux:
	cd js/node-manager && \
		source ~/.nvm/nvm.sh && \
		nvm install && \
		npm install && \
		npm version $(ACTYX_VERSION_NODEMANAGER) && \
		npm run build && \
		npm run dist && \
		npm run artifacts


# combines all the .so files to build actyxos on android
android-libaxosnodeffi: \
	jvm/os-android/app/src/main/jniLibs/x86/libaxosnodeffi.so \
	jvm/os-android/app/src/main/jniLibs/x86_64/libaxosnodeffi.so \
	jvm/os-android/app/src/main/jniLibs/arm64-v8a/libaxosnodeffi.so \
	jvm/os-android/app/src/main/jniLibs/armeabi-v7a/libaxosnodeffi.so

jvm/os-android/app/src/main/jniLibs/x86/libaxosnodeffi.so: rust/actyx/target/i686-linux-android/release/libaxosnodeffi.so
	mkdir -p $(dir $@)
	cp $< $@

jvm/os-android/app/src/main/jniLibs/x86_64/libaxosnodeffi.so: rust/actyx/target/x86_64-linux-android/release/libaxosnodeffi.so
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
	rm -f $@
	mv $< $@
# here % (and thus $*) matches something like ax.exe, so we need to strip the suffix with `basename`
rust/actyx/target/release/%: make-always
	cd rust/actyx && $(CARGO) --locked build --release -j $(CARGO_BUILD_JOBS) --bin $(basename $*)

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
	rm -f $$@
	mv $$< $$@
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
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  $(DOCKER_FLAGS) \
	  $(image-$(word 3,$(subst -, ,$(TARGET)))) \
	  cargo +$(BUILD_RUST_TOOLCHAIN) --locked build --release -j $(CARGO_BUILD_JOBS) $(CARGO_BUILD_ARGS) --bin $$(basename $$*) --target $(TARGET)
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
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v $(CARGO_HOME)/git:/home/builder/.cargo/git \
	  -v $(CARGO_HOME)/registry:/home/builder/.cargo/registry \
	  --rm \
	  $(DOCKER_FLAGS) \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
	  cargo +$(BUILD_RUST_TOOLCHAIN) --locked build -p node-ffi --lib --release -j $(CARGO_BUILD_JOBS) $(CARGO_BUILD_ARGS) --target $(TARGET)

# create these so that they belong to the current user (Docker would create as root)
# (formulating as rule dependencies only runs mkdir when they are missing)
cargo-init: $(CARGO_HOME)/git $(CARGO_HOME)/registry
$(CARGO_HOME)/%:
	mkdir -p $@

jvm/os-android/app/build/outputs/bundle/release/app-release.aab: android-libaxosnodeffi make-always
	jvm/os-android/bin/get-keystore.sh
	docker run \
	  -u builder \
	  -v `pwd`:/src \
	  -w /src/jvm/os-android \
	  --rm \
	  $(DOCKER_FLAGS) \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
      ./gradlew --stacktrace ktlintCheck build bundleRelease

dist/bin/actyx.apk: jvm/os-android/app/build/outputs/bundle/release/app-release.aab make-always
	jvm/os-android/bin/get-keystore.sh
	rm -f dist/bin/actyx.apks
	docker run \
	  -u builder \
	  -v `pwd`:/src \
	  -w /src/jvm/os-android \
	  --rm \
	  $(DOCKER_FLAGS) \
	  actyx/util:buildrs-x64-$(IMAGE_VERSION) \
      java -jar /usr/local/lib/bundletool.jar build-apks \
				--bundle /src/$< \
				--output=/src/dist/bin/actyx.apks \
				--ks=/src/jvm/os-android/actyx-local/axosandroid.jks \
				--ks-key-alias=axosandroid \
				--ks-pass=pass:$(shell grep actyxKeyPassword jvm/os-android/actyx-local/actyx.properties|cut -f2 -d\") \
				--mode=universal
	unzip -o dist/bin/actyx.apks universal.apk
	mv -f universal.apk dist/bin/actyx.apk

dist/bin/actyx.aab: jvm/os-android/app/build/outputs/bundle/release/app-release.aab
	mkdir -p $(dir $@)
	cp $< $@


dist/bin/windows-x86_64/actyx-x64.msi: dist/bin/windows-x86_64/actyx.exe make-always
	docker run \
	  -v `pwd`:/src \
	  -e WIN_CODESIGN_CERTIFICATE \
	  -e WIN_CODESIGN_PASSWORD \
	  --rm \
	  actyx/util:actyx-win-installer-builder \
	  bash /src/wix/actyx-installer/build.sh ${ACTYX_VERSION} "/src/dist/bin/windows-x86_64"

define mkDockerRule =
docker-$(1):
	docker buildx build \
	  --platform $(docker-platform-$(1)) \
	  $(docker-build-args) \
	  -f docker/actyx/Dockerfile \
	  --tag actyx/actyx-ci:actyx-$(1)-$(GIT_COMMIT) \
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
	  -f docker/actyx/Dockerfile \
	  .

# build for local architecture and load into docker daemon
docker-current:
	docker buildx build --load $(docker-build-args) -f docker/actyx/Dockerfile .

# This is here to ensure that we use the same build-args here and in artifacts.yml
docker-multiarch-build-args:
	@echo $(docker-multiarch-build-args)

docker-push-actyx:
	docker buildx build \
		$(docker-multiarch-build-args) \
		--push \
		--tag actyx/actyx-ci:actyx-$(GIT_COMMIT) $(ADDITIONAL_DOCKER_ARGS) \
		-f docker/actyx/Dockerfile \
		.
