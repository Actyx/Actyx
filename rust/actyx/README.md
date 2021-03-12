# Rust based core infrastructure

_for a map of the modules and components, see below_

## Building

This is a multi-project build, so running `cargo` in this top level directory will build every subproject. To build or
test a specific subproject you can you the `-p` option:

```
  cargo build -p ipfs
```

## Testing

```
  cargo run test
```

### Integration tests using docker

_Note: This features does not work with a virtual workspace (https://github.com/rust-lang/cargo/issues/4942). So you need to run the tests from the specific Cargo workspace the tests are defined.
For your convenience, there is a [script](./run-docker-integration-tests.sh) to automate that._

To also enable integration tests, that spin up docker containers for testing purposes, run:
```
  cargo run test --features docker-integration
```

This works by prefixing your tests like so:
```rs
#[test]
#[cfg_attr(not(feature = "docker-integration"), ignore)]
fn should_find_provs() {
```
which means, that the test will be ignored if the _docker-integration_ feature is not enabled (which by default, is not).

The relevant `Cargo.toml` (of the individual project) needs to be augmented with ..
```toml
[features]
docker-integration = []
```

# Rust futures runtime considerations

In cases where we need a futures runtime, we default to tokio 0.2. If possible, executables
should use `#[tokio::main]`. Tests using futures can use `#[tokio::test]` for convenience.

Spawning a task from the environment using `tokio::spawn` should be avoided except in `async fn` or `async` blocks. If this is not possible, please document the rationale.

Services or runtimes should be spawned close to or in `main()`, with a comment explaning the usage of
`tokio::spawn` or an explicit separate thread pool.

# Map of our components

## Event Service

- started by Store (calling `async fn run_event_service`)
- calling methods on `struct LocalStore` (owned)
- calling methods on `ConsumerAccess` (passed in)

## Pond Service

_basically like the Event Service_

## The Store

crates:

- `store`: implements `Store::run` and contains some loops
- `store-cli`: binary for starting the store via `async fn Store::run`
- `store-core`: main functionality
- `store-lib`: FFI bindings for old Android shell app, using `async fn Store::run` — deprecated, to be removed

interfaces:

- started/stopped by Node’s `StoreHandler::run` (creating runtime, then calling `async fn Store::run`; lifecycle monitoring?)
- `ax-config::StoreConfig`

## Android

crates:

- `node-ffi`: FFI bindings for `node`, to be used from Android (Kotlin)

interfaces:

- started by host interface via `axnode_init` (takes workdir and callback)
- stopped by host interface via `shutdown`

## Docker

contained in crate `ax-os-node`

- started by host interface via `DockerServiceHandler::run`, reacts to config changes
- sends back runtime events back to Node via crossbeam channel

## Console Service

contained in crate `ax-os-node`

- HTTP port for `ax` CLI and submitting logs
- started by host interface
- feeds Node/App FSMs
- sends logs to logsvcd

## `logsvcd`

interfaces:

- started by host interface (blocking call)
- internal API for writing and reading logs, via crossbeam channel

## IPFS

interfaces:

- started by host interface currently
    - Android starts go-ipfs process
    - Docker starts Python wrapper that starts go-ipfs based on watching a config file
- Store creates the right `trait Ipfs` implementation based on internal/external IPFS implementation

## The Node manager

contained in crate `ax-os-node`

- started by host interface via `NodeLifeCycleWrapper::new` (starts its own thread with `Node::run`)
- gets external events from host interface

# General

## Store::run refactoring

There are many parts that should be moved into the Node, like creating the IPFS interface.

## NodeRuntimeStorage

- contains translation between AppFsm and host interface
