use actyx_sdk::{Ax, AxOpts};
use futures::StreamExt;

// This example demonstrates how to subscribe to a query.
#[tokio::main]
async fn main() {
    // Setup a default Ax client with default settings.
    let service = Ax::new(AxOpts::default()).await.unwrap();

    let offsets = service.offsets().await.unwrap().present;

    // Start a subscription, receiving only events tagged with `example:tag`
    let mut subscribe_response = service
        .subscribe_monotonic("FROM 'sensor:temp-sensor1'")
        .with_lower_bound(offsets)
        .await
        .unwrap();

    // Consume events from the subscription stream
    while let Some(response) = subscribe_response.next().await {
        println!("{:?}", response)
    }
}
