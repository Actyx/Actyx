use ax_sdk::{service::QueryResponse, tags, Ax, AxOpts};
use futures::stream::StreamExt;

// This example demonstrates how to query events using `with_lower_bound`.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a default Ax client with default settings.
    let service = Ax::new(AxOpts::default()).await?;

    // Publish the events
    let publish_response = service
        .publish()
        .event(
            tags!("temperature", "sensor:temp-sensor1"),
            &serde_json::json!({ "temperature": 10 }),
        )?
        .event(
            tags!("temperature", "sensor:temp-sensor2"),
            &serde_json::json!({ "temperature": 21 }),
        )?
        .event(
            tags!("temperature", "sensor:temp-sensor3"),
            &serde_json::json!({ "temperature": 40 }),
        )?
        .await?;
    // Print publish response for demonstration purposes
    println!("{:?}", publish_response);

    let mut query_response = service.query("FROM 'sensor:temp-sensor2'").await?;
    let offsets = loop {
        let result = query_response.next().await.unwrap();
        if let QueryResponse::Offsets(offsets) = result {
            break offsets.offsets;
        }
    };

    // Print the received offsets for demonstration purposes
    println!("{:?}", offsets);

    // You can also use `with_upper_bound` here to query "up to"
    let mut query_response = service
        .query("FROM 'temperature'")
        .with_lower_bound(offsets.clone())
        .await?;
    while let Some(response) = query_response.next().await {
        println!("{:?}", response);
    }

    Ok(())
}
