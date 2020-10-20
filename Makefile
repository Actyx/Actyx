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
	cargo clean --manifest-path rt-master/Cargo.toml

# mark things with this dependency to run whenever requested
.PHONY: ALWAYS

prepare: ALWAYS
	rustup default $(BUILD_RUST_TOOLCHAIN)
	docker pull actyx/util:buildnode-x64-$(IMAGE_VERSION)
	docker pull actyx/util:buildrs-x64-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-aarch64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-x86_64-unknown-linux-musl-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-armv7-unknown-linux-musleabihf-$(IMAGE_VERSION)
	docker pull actyx/cosmos:musl-arm-unknown-linux-musleabi-$(IMAGE_VERSION)

# define mapping from os-arch to target
target-linux-aarch64 = aarch64-unknown-linux-musl
target-linux-x86_64 = x86_64-unknown-linux-musl
target-linux-armv7 = armv7-unknown-linux-musleabihf
target-linux-arm = arm-unknown-linux-musleabi
target-windows-x86_64 = x86_64-pc-windows-gnu

# define mapping from os to image name
image-linux = actyx/cosmos:musl-$(TARGET)-$(IMAGE_VERSION)
image-windows = actyx/util:buildrs-x64-$(IMAGE_VERSION)

dist/bin/%: ARTIFACT = rt-master/target/$(value target-$(word 1,$(subst /, ,$*)))/release/$(word 2,$(subst /, ,$*))
dist/bin/%: ALWAYS
	make $(ARTIFACT)
	mkdir -p $(dir $@)
	cp $(ARTIFACT) $@

rt-master/target/%: TARGET = $(word 1,$(subst /, ,$*))
rt-master/target/%: OS = $(word 3,$(subst -, ,$(TARGET)))
rt-master/target/%: BIN = $(basename $(word 3,$(subst /, ,$*)))
rt-master/target/%: ALWAYS
	@# create these so that they belong to the current user (Docker would create as root)
	mkdir -p ${CARGO_HOME}/git
	mkdir -p ${CARGO_HOME}/registry
	docker run \
	  -u $(shell id -u) \
	  -w /src/rt-master \
	  -e SCCACHE_REDIS=$(SCCACHE_REDIS) \
	  -e CARGO_BUILD_TARGET=$(TARGET) \
	  -e CARGO_BUILD_JOBS=8 \
	  -v `pwd`:/src \
	  -v ${CARGO_HOME}/git:/home/builder/.cargo/git \
	  -v ${CARGO_HOME}/registry:/home/builder/.cargo/registry \
	  -it \
	  $(image-$(OS)) \
	  cargo --locked build --release --bin $(BIN)

