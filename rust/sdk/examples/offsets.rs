#[allow(unused_imports)]
#[macro_use]
extern crate actyxos_sdk_macros;

use actyxos_sdk::{
    event_service::{EventService, Order, QueryRequest, QueryResponse},
    tagged::EventServiceHttpClient,
};
use futures::stream::StreamExt;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // client for locally running Actyx Event Service
    let service = EventServiceHttpClient::default();

    // retrieve largest currently known event stream cursor
    let offsets = service.offsets().await?;

    // all events matching the given subscription
    // sorted backwards, i.e. youngest to oldest
    let mut events = service
        .query(QueryRequest {
            lower_bound: None,
            upper_bound: offsets,
            r#where: "MyFish".parse()?,
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
