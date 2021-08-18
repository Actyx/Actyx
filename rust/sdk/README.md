[![Latest Version](https://img.shields.io/crates/v/actyx_sdk.svg)](https://crates.io/crates/actyx_sdk)
[![Rust Documentation](https://docs.rs/actyx_sdk/badge.svg)](https://docs.rs/actyx_sdk)

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> Actyx SDK

[Actyx](https://developer.actyx.com/) is a decentralized event database, streaming and processing
engine that allows you to easily build [local-first cooperative](https://www.local-first-cooperation.org/)
apps. It makes it easy to run distributed
applications on multiple nodes. It is a piece of software that allows you to run your own apps
on one or more edge devices and have these apps seamlessly communicate and share data with
each other.

This crate defines the data types needed for communicating with Actyx and provides Rust
bindings for the Actyx APIs. It also provides serialization instances for processing the
events with [`differential-dataflow`](https://docs.rs/differential-dataflow) under the `"dataflow"`
[feature flag](#feature-flags).

# Examples

Below you find a full example using the [`EventService`](service/trait.EventService.html)
client that retrieves some events. Please adapt the queried tags to match your stored events
in order to see output.

> _Note: this example needs the `client` feature to compile._

```no_run
use actyx_sdk::{
  app_id, AppManifest, HttpClient,
  service::{EventService, Order, QueryRequest, QueryResponse},
};
use futures::stream::StreamExt;
use url::Url;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
  // add your app manifest, for brevity we will use one in trial mode
  let app_manifest = AppManifest::new(
      app_id!("com.example.my-awesome-app"),
      "display name".into(),
      "0.1.0".into(),
      None,
  );

  // Url of the locally running Actyx node
  let url = Url::parse("http://localhost:4454")?;
  // create client for it
  let service = HttpClient::new(url, app_manifest).await?;

  // all events matching the given subscription
  // sorted backwards, i.e. youngest to oldest
  let mut events = service
      .query(QueryRequest {
          lower_bound: None,
          upper_bound: None,
          query: "FROM 'MyFish'".parse()?,
          order: Order::Desc,
      })
      .await?;

  // print out the payload of each event
  // (cf. Payload::extract for more options)
  while let Some(QueryResponse::Event(event)) = events.next().await {
      println!("{}", event.payload.json_value());
  }
  Ok(())
}
```

# Feature flags

The default is to provide only the data types with serialization and deserialization support
for [`serde`](https://docs.rs/serde). The following features can be enabled in addition:

- `client`: include HTTP client bindings using the [`reqwest`](https://docs.rs/reqwest) crate
- `dataflow`: provide [`Abomonation`](https://docs.rs/abomonation) instances for use with tools
  like [`Differential Dataflow`](https://docs.rs/differential-dataflow)
- `arb`: provide
  [`quickcheck::Arbitrary`](https://docs.rs/quickcheck/latest/quickcheck/trait.Arbitrary.html)
  instances for common data types. This is useful for testing.
