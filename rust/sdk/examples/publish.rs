use actyx_sdk::{service::QueryResponse, tags, Ax, AxOpts, Offset};
use futures::stream::StreamExt;

// This example demonstrates how to publish events using the higher-level API.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a default Ax client with default settings.
    let service = Ax::new(AxOpts::default()).await?;

    // Publish the events
    let publish_response = service
        .publish()
        // Serialization may fail and thus the `.event` call returns a `Result`
        .event(
            tags!("temperature", "sensor:temp-sensor1"),
            &serde_json::json!({ "temperature": 10 }),
        )?
        // Multiple calls to `.event` are supported and all events will be published
        .event(
            tags!("temperature", "sensor:temp-sensor2"),
            &serde_json::json!({ "temperature": 21 }),
        )?
        .await?;

    let mut query_response = service.query("FROM 'sensor:temp-sensor2'").await?;
    let offsets = loop {
        let result = query_response.next().await.unwrap();
        if let QueryResponse::Offsets(offsets) = result {
            break offsets.offsets;
        }
    };

    println!("{:?}", offsets);

    let mut query_response = service
        .query("FROM 'temperature'")
        .with_lower_bound(offsets.clone())
        .await?;
    while let Some(response) = query_response.next().await {
        println!("{:?}", response);
    }

    // Print the response
    println!("{:?}", publish_response);
    Ok(())
}
