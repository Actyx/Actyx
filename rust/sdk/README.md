[![Latest Version](https://img.shields.io/crates/v/actyxos_sdk.svg)](https://crates.io/crates/actyxos_sdk)
[![Rust Documentation](https://docs.rs/actyxos_sdk.svg)](https://docs.rs/actyxos_sdk)

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> ActyxOS SDK

[ActyxOS](https://developer.actyx.com/docs/os/introduction) makes it easy to run distributed
applications on multiple nodes. It is a piece of software that allows you to run your own apps
on one or more edge devices and have these apps seamlessly communicate and share data with
each other.

This crate defines the data types needed for communicating with ActyxOS and provides Rust
bindings for the ActyxOS APIs.

## Examples

Below you find a full example using the [`EventService`](https://docs.rs/actyxos_sdk/latest/actyxos_sdk/event_service/struct.EventService.html)
client that retrieves some events. Please adapt the `semantics` to match your stored events
in order to see output.

> _Note: (this example needs the `client` feature to compile)_

```rust
use actyxos_sdk::event_service::{EventService, EventServiceError, Order, Subscription};
use futures::stream::StreamExt;

#[tokio::main]
pub async fn main() -> Result<(), EventServiceError> {
    // create a client to the locally running ActyxOS Event Service
    let service = EventService::default();

    // retrieve largest currently known event stream cursor
    let offsets = service.get_offsets().await?;

    // ask for all events matching the given subscription from now backwards
    let mut events = service
        .query_upto(
            offsets,
            vec![Subscription::semantics("edge.ax.sf.Terminal".into())],
            Order::LamportReverse,
        )
        .await?;

    // print out the payload of each event (cf. Payload::extract for more options)
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
