[![Latest Version](https://img.shields.io/crates/v/actyx_sdk.svg)](https://crates.io/crates/actyx_sdk)
[![Rust Documentation](https://docs.rs/actyx_sdk/badge.svg)](https://docs.rs/actyx_sdk)

# <img src="https://developer.actyx.com/images/rust-sdk.png" height="32px"> Actyx SDK

[Actyx](https://developer.actyx.com/docs/os/introduction) makes it easy to run distributed
applications on multiple nodes. It is a piece of software that allows you to run your own apps
on one or more edge devices and have these apps seamlessly communicate and share data with
each other.

This crate defines the data types needed for communicating with Actyx and provides Rust
bindings for the Actyx APIs.

## Examples

Below you find a full example using the [`EventService`](https://docs.rs/actyx_sdk/latest/actyx_sdk/event_service/struct.EventService.html)
client that retrieves some events. Please adapt the `semantics` to match your stored events
in order to see output.

> _Note: (this example needs the `client` feature to compile)_

```rust
use actyx_sdk::event_service::{EventService,
        EventServiceError, Order, Subscription};
use futures::stream::StreamExt;

#[tokio::main]
pub async fn main() -> Result<(), EventServiceError> {
    // client for locally running Actyx Event Service
    let service = EventService::default();

    // retrieve largest currently known event stream cursor
    let offsets = service.get_offsets().await?;

    // all events matching the given subscription
    // sorted backwards, i.e. youngest to oldest
    let sub = vec![Subscription::semantics("MyFish")];
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

## Feature flags

The default is to provide only the data types with serialization and deserialization support
for [`serde`](https://docs.rs/serde). The following features can be enabled in addition:

- `client`: include HTTP client bindings using the [`reqwest`](https://docs.rs/reqwest) crate
- `dataflow`: provide [`Abomonation`](https://docs.rs/abomonation) instances for use with tools
  like [`Differential Dataflow`](https://docs.rs/differential-dataflow)
