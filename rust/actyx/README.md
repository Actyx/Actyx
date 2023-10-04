# Rust based core infrastructure

## Building

This is a multi-project build, so running `cargo` in this top level directory will build every subproject. To build or
test a specific subproject you can you the `-p` option:

```sh
  cargo build -p ipfs
```

## Testing

```sh
  cargo test
```

## Map of our components

All data types used across serialisation boundaries should be moved to the `SDK`, which removes the need for `*-formats` crates.
Local storage is managed by the [`ipfs-sqlite-block-store`](https://github.com/Actyx/ipfs-sqlite-block-store) crate.
Replicated storage is managed by the [`banyan`](https://github.com/Actyx/banyan) crate.
IPFS swarm behaviour is implemented in the [`ipfs-embed`](https://github.com/ipfs-rust/ipfs-embed) crate.
Then, we need the following:

- `trees`: holds Banyan adapters and data types (e.g. tag index and associated logic); offers interface for adding events, updating forests and for subscribing to filtered single streams (i.e. tree-level query “language” is defined here as well); `trees` is a passive crate, only providing functions to compute new things (including stream transformers)
- `crypto`: key store, token store, signing & validation, no dependencies
- `swarm`: libp2p network behavior with gossip, bitswap, NodeId; offers interface to publish heartbeats and forest updates or subscribe to incoming traffic, owns the Lamport clock, needs `crypto`; `swarm` is an active crate that manages network activities and introduces new inputs into the whole program, interacts with `trees` and the block store
- `runtime`: AQL expression evaluation based on access to `trees`
- `api`: HTTP endpoints, providing their services based on `crypto`, `runtime`, `routing`, `trees`, and `swarm` (for Lamport timestamps — we may also inject a `punch_clock` function to achieve this)
- `node`: startup procedures for the supported operating systems, managing the block store and its ephemeral stream expiry, reading/creating NodeId and initial Lamport clock on startup; ensures that panic (anywhere!) will be reported and will terminate the process; libp2p network behavior for talking with Actyx CLI; using `trees` and `runtime` for settings and logs, internally manages list of authorised keys
- `settings`: storage and management of Actyx settings using sqlite
- `certs`: tools for creating keypairs and licenses
- `util`, `ax-futures-util`: various extension traits for streams, futures, logging, etc.
- `libp2p-streaming-response`: the larger brother to libp2p-request-response; sends many responses in the same substream, instead of using oneshot handlers either way

In general, data types needed within each of these should be defined in the respective crates.
