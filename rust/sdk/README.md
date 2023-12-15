[![Latest Version](https://img.shields.io/crates/v/ax_sdk.svg)](https://crates.io/crates/ax_sdk)
[![Rust Documentation](https://docs.rs/ax_sdk/badge.svg)](https://docs.rs/ax_sdk)

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> AX SDK

[AX](https://crates.io/crates/ax) is a decentralized event database, streaming and processing
engine that allows you to easily build [local-first cooperative](https://www.local-first-cooperation.org/)
apps.
It makes it easy to run distributed applications on multiple nodes.
It is a piece of software that allows you to run your own apps on one or more edge devices and have these apps seamlessly communicate and share data with each other.

---

> **Note:** _AX has been created by the company [Actyx AG](https://developer.actyx.com/) and predates the unrelated [Actix framework](https://crates.io/crates/actix) (although AX wasn’t open-sourced until Oct’23).
> While we have changed crate names to avoid confusion, the company name remains Actyx AG and appears in some places._

---

This crate defines the data types needed for communicating with Actyx and provides Rust
bindings for the AX APIs.

# Examples

Below you find a full example using the [`Ax`](struct.Ax.html)
client that retrieves some events. Please adapt the queried tags to match your stored events
in order to see output.

```no_run
use ax_sdk::{
  Ax, AxOpts,
  types::{
    app_id, AppManifest,
    service::{Order, QueryRequest, QueryResponse},
  }
};
use futures::stream::StreamExt;
use url::Url;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
  // Add your app manifest, for brevity we will use one in trial mode
  let manifest = AppManifest::trial(
      app_id!("com.example.my-awesome-app"),
      "display name".into(),
      "0.1.0".into(),
  ).unwrap();

  // Url of the locally running Actyx node
  let url = Url::parse("http://localhost:4454")?;

  // Create client for it
  let service = Ax::new(AxOpts { url, manifest }).await?;

  // all events matching the given subscription
  // sorted backwards, i.e. youngest to oldest
  let mut events = service
      .query("FROM 'MyTag'")
      .with_order(Order::Desc)
      .await?;

  // print out the payload of each event
  // (cf. `Payload::extract` for more options)
  while let Some(QueryResponse::Event(event)) = events.next().await {
      println!("{}", event.payload.json_value());
  }
  Ok(())
}
```

# Feature flags

- `arb`: provide
  [`quickcheck::Arbitrary`](https://docs.rs/quickcheck/latest/quickcheck/trait.Arbitrary.html)
  instances for common data types. This is useful for testing.
