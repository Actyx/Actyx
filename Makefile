SHELL := /bin/bash

all-LINUX := $(foreach arch,x86_64 aarch64 armv7 arm,linux-$(arch)/actyxos-linux)
all-WINDOWS := windows-x86_64/win.exe

all: $(patsubst %,dist/bin/%,$(all-LINUX) $(all-WINDOWS))

# These should be moved to the global azure pipelines build
export BUILD_RUST_TOOLCHAIN := 1.45.0
export BUILD_SCCACHE_VERSION := 0.2.12

export CARGO_HOME ?= $(HOME)/.cargo

export SCCACHE_REDIS ?= $(shell vault kv get -field=SCCACHE_REDIS secret/ops.actyx.redis-sccache)

# Helper to try out local builds of Docker images
export IMAGE_VERSION := $(or $(LOCAL_IMAGE_VERSION),latest)

# Debug helpers
print-%:
	@echo $* = $($*)

clean:
	cd rt-master && cargo clean

# mark things with this dependency to run whenever requested
.PHONY: UNCONDITIONAL

prepare: UNCONDITIONAL
	rustup default $(BUILD_RUST_TOOLCHAIN)
	# used for js builds
	docker pull actyx/util:buildnode-x64-$(IMAGE_VERSION)
	# used for windows rust builds
	docker pull actyx/util:buildrs-x64-$(IMAGE_VERSION)
	# used for linux rust builds
	docker pull actyx/cosmos:musl-aarch64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-x86_64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-armv7-unknown-linux-musleabihf-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-arm-unknown-linux-musleabi-$(IMAGE_VERSION)

# list all os-arch and binary names
osArch = linux-aarch64 linux-x86_64 linux-armv7 linux-arm windows-x86_64
binaries = ax ax.exe actyxos-linux win.exe

# compute list of all OSs and rust targets
os = $(sort $(foreach oa,$(osArch),$(word 1,$(subst -, ,$(oa)))))
targets = $(sort $(foreach oa,$(osArch),$(target-$(oa))))

# execute linter, style checker and tests for everything
validate: validate-rt-master validate-rust-sdk validate-rust-sdk-macros validate-os-android validate-js validate-website

# declare all the validate targets to be phony
.PHONY: validate-rt-master validate-rust-sdk validate-rust-sdk-macros validate-os-android validate-js validate-website

CARGO := cargo +$(BUILD_RUST_TOOLCHAIN)

# execute fmt check, clippy and tests for rt-master
validate-rt-master:
	cd rt-master && \
		$(CARGO) fmt --all -- --check && \
		$(CARGO) --locked clippy -- -D warnings && \
		$(CARGO) --locked clippy --tests -- -D warnings && \
		$(CARGO) test --all-features

# execute fmt check, clippy and tests for rust-sdk
validate-rust-sdk:
	cd rust/sdk && \
		$(CARGO) fmt --all -- --check && \
		$(CARGO) --locked clippy -- -D warnings && \
		$(CARGO) --locked clippy --tests -- -D warnings && \
		$(CARGO) test --all-features

# execute fmt check, clippy and tests for rust-sdk-macros
validate-rust-sdk-macros:
	cd rust/sdk_macros && \
		$(CARGO) fmt --all -- --check && \
		$(CARGO) --locked clippy -- -D warnings && \
		$(CARGO) --locked clippy --tests -- -D warnings && \
		$(CARGO) test --all-features

# execute linter for os-android
validate-os-android:
	cd jvm/os-android/ && \
		./gradlew clean ktlintCheck test

# validate all js
validate-js: validate-js-pond validate-js-sdk

# validate js pond
validate-js-pond:
	cd js/pond && \
		npm i && \
		npm run test && \
		npm run build:prod

# validate js sdk
validate-js-sdk:
	cd js/os-sdk && \
		npm i && \
		npm run test && \
		npm run build

# validate all websites
validate-website: validate-website-developer validate-website-downloads

# validate developer.actyx.com
validate-website-developer:
	cd web/developer.actyx.com && \
		npm i && \
		npm run test

# validate downloads.actyx.com
validate-website-downloads:
	cd web/downloads.actyx.com && \
		npm i

# define mapping from os-arch to target
target-linux-aarch64 = aarch64-unknown-linux-musl
target-linux-x86_64 = x86_64-unknown-linux-musl
target-linux-armv7 = armv7-unknown-linux-musleabihf
target-linux-arm = arm-unknown-linux-musleabi
target-windows-x86_64 = x86_64-pc-windows-gnu

# define mapping from os to builder image name
image-linux = actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION)
image-windows = actyx/util:buildrs-x64-$(IMAGE_VERSION)

# build rules for binaries on the current platform (i.e. no cross-building)
dist/bin/current/%: rt-master/target/release/%
	mkdir -p $(dir $@)
	cp $< $@
rt-master/target/release/%: UNCONDITIONAL
	cd rt-master && cargo --locked build --release --bin $(basename $*)

# define build rules for all cross-built binaries (unfortunately using pattern rules is impossible)
define mkDistRule =
dist/bin/$(1)/$(2): rt-master/target/$(target-$(1))/release/$(2)
	mkdir -p $$(dir $$@)
	cp $$< $$@
endef
$(foreach oa,$(osArch),$(foreach bin,$(binaries),$(eval $(call mkDistRule,$(oa),$(bin)))))

# make a list of pattern rules (with %) for all possible rust binaries
targetPatterns = $(foreach t,$(targets),rt-master/target/$(t)/release/%)

$(targetPatterns): TARGET = $(word 3,$(subst /, ,$@))
$(targetPatterns): OS = $(word 3,$(subst -, ,$(TARGET)))
$(targetPatterns): UNCONDITIONAL
	@# create these so that they belong to the current user (Docker would create as root)
	mkdir -p ${CARGO_HOME}/git
	mkdir -p ${CARGO_HOME}/registry
	docker run \
	  -u $(shell id -u) \
	  -w /src/rt-master \
	  -e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=8 \
	  -e HOME=/home/builder \
	  -v `pwd`:/src \
	  -v ${CARGO_HOME}/git:/home/builder/.cargo/git \
	  -v ${CARGO_HOME}/registry:/home/builder/.cargo/registry \
	  -it --rm \
	  $(image-$(OS)) \
	  cargo --locked build --release --bin $(basename $*)

