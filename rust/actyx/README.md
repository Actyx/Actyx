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

_Note: This feature does not work with a virtual workspace (<https://github.com/rust-lang/cargo/issues/4942>). So you need to run the tests from the specific Cargo workspace the tests are defined.
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

All data types used across serialisation boundaries should be moved to the `SDK`, which removes the need for `*-formats` crates. Local storage is managed by the `ipfs-sqlite-block-store` crate. Then, we need the following:

- `trees`: holds Banyan adapters and data types (e.g. tag index and associated logic); offers interface for adding events, updating forests and for subscribing to filtered single streams (i.e. tree-level query “language” is defined here as well); `trees` is a passive crate, only providing functions to compute new things (including stream transformers)
- `crypto`: key store, token store, signing & validation, no dependencies
- `swarm`: libp2p network behavior with gossip, bitswap, NodeId; offers interface to publish heartbeats and forest updates or subscribe to incoming traffic, owns the Lamport clock, needs `crypto`; `swarm` is an active crate that manages network activities and introduces new inputs into the whole program, interacts with `trees` and the block store
- `query`: tag expression language incl. translation into executable form, no dependencies other than parser library
- `runtime`: tag expression evaluation based on access to `trees`, yielding streams of records/events, uses `query`
- `routing`: config-based rule engine to place events into streams based on appId and tags, depends on `trees` for persistent stream number management
- `api`: HTTP endpoints, providing their services based on `crypto`, `runtime`, `routing`, `trees`, and `swarm` (for Lamport timestamps — we may also inject a `punch_clock` function to achieve this)
- `admin`: libp2p network behavior for talking with Actyx CLI; using `trees` and `runtime` for settings and logs, internally manages list of authorised keys
- `node`: startup procedures for the supported operating systems, managing the block store and its ephemeral stream expiry, reading/creating NodeId and initial Lamport clock on startup; ensures that panic (anywhere!) will be reported and will terminate the process

In general, data types needed within each of these should be defined in the respective crates.
