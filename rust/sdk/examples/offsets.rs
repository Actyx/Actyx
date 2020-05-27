#[allow(unused_imports)]
#[macro_use]
extern crate actyxos_sdk_macros;

use actyxos_sdk::{
    event_service::{EventService, EventServiceError, Order, Subscription},
    semantics,
};
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
