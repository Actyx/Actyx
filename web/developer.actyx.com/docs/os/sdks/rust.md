---
title: Rust SDK
---

<!-- Add as react component to be able to handle the width (otherwise it goes full width) -->
<img src="/images/rust-sdk.png" style={{maxWidth: "150px", marginBottom: "2rem" }} />

Building apps in [Rust](https://www.rust-lang.org/) and want to easily create and access data streams in your ActyxOS swarm? That's what we built the ActyxOS SDK for Rust for. The [`actyxos_sdk` crate](https://crates.io/crates/actyxos_sdk) defines all necessary data types and provides Rust bindings for communicating with ActyxOS's [Event Service API](../api/event-service.md).

## Installation

Add the following dependency to your `Cargo.toml` file:

```yml
[dependencies]
actyxos_sdk = "~0.2"
```

## Example

Here is an example using the `EventService` client that retrieves some events. Please adapt the `semantics` to match your stored events in order to see output.

```rust
use actyxos_sdk::event_service::{EventService, EventServiceError, Order, Subscription};
use actyxos_sdk::semantics;
use futures::stream::StreamExt;

#[tokio::main]
pub async fn main() -> Result<(), EventServiceError> {
    // client for locally running ActyxOS Event Service
    let service = EventService::default();

    // retrieve largest currently known event stream cursor
    let offsets = service.get_offsets().await?;

    // all events matching the given subscription
    // sorted backwards, i.e. youngest to oldest
    let sub = vec![Subscription::wildcard(semantics!("MyFish"))];
    let mut events = service
        .query_upto(offsets, sub, Order::LamportReverse)
        .await?;

    // print out the payload of each event
    // (cf. Payload::extract for more options)
    while let Some(event) = events.next().await {
        println!("{}", event.payload.json_value());
    }
    Ok(())
}
```

## Documentation

You can find the latest documentation for the Rust SDK [on docs.rs](https://docs.rs/actyxos_sdk).
