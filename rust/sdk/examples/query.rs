use actyx_sdk::{service::Order, Ax, AxOpts};
use futures::stream::StreamExt;

// This example demonstrates how to query events.
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // Setup a default Ax client with default settings.
    let service = Ax::new(AxOpts::default()).await?;

    // Create a query
    let mut events = service
        .query("FROM 'sensor:temp-sensor'")
        // Set the order for the received results
        .with_order(Order::Desc)
        .with_lower_bound(service.offsets().await?.present)
        .await?;

    // Consume the query result stream
    while let Some(event) = events.next().await {
        println!("{:?}", event);
    }
    Ok(())
}
