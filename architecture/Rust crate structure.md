# Rust crate structure

All data types used across serialisation boundaries as well as AQL parsing should be found in the [`SDK`](../rust/sdk), which removes the need for `*-formats` crates.
Local storage is managed by the [`ipfs-sqlite-block-store`](https://github.com/Actyx/ipfs-sqlite-block-store) crate.
Replicated storage is managed by the [`banyan`](https://github.com/Actyx/banyan) crate.

This leaves the following crates in our codebase:

- `trees`: holds Banyan adapters and data types (e.g. tag index and associated logic); offers interface for adding events, updating forests and for subscribing to filtered single streams (i.e. tree-level query “language” is defined here as well); `trees` is a passive crate, only providing functions to compute new things (including stream transformers)
- `crypto`: key store, signing & validation, no dependencies
- `swarm`: libp2p network behavior with gossip, bitswap, NodeId; offers interface to publish heartbeats and forest updates or subscribe to incoming traffic, owns the Lamport clock, needs `crypto`; `swarm` is an active crate that manages network activities and introduces new inputs into the whole program, interacts with `trees` and the block store
- `runtime`: AQL evaluation based on access to `trees`, yielding streams of records/events, uses `query`
- `api`: HTTP endpoints, providing their services based on `crypto`, `runtime`, `routing`, `trees`, and `swarm` (for Lamport timestamps — we may also inject a `punch_clock` function to achieve this)
- `admin`: libp2p network behavior for talking with Actyx CLI; using `trees` and `runtime` for settings and logs, internally manages list of authorised keys
- `node`: startup procedures for the supported operating systems, managing the block store and its ephemeral stream expiry, reading/creating NodeId and initial Lamport clock on startup; ensures that panic (anywhere!) will be reported and will terminate the process

In general, data types needed within each of these should be defined in the respective crates.
