use ax_sdk::{
    types::{service::PublishEvent, Payload},
    Ax, AxOpts,
};
use ax_types::tags;

// This example demonstrates how to publish events by constructing them
// with their low-level API
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a default Ax client with default settings
    let service = Ax::new(AxOpts::default()).await?;
    // Publish the events
    let publish_response = service
        .publish()
        // Manually create two events
        .events([
            PublishEvent {
                tags: tags!("temperature", "sensor:temp-sensor1"),
                payload: Payload::compact(&serde_json::json!({ "temperature": 10 })).unwrap(),
            },
            PublishEvent {
                tags: tags!("temperature", "sensor:temp-sensor2"),
                payload: Payload::compact(&serde_json::json!({ "temperature": 27 })).unwrap(),
            },
        ])
        .await?;
    // Print the response
    println!("{:?}", publish_response);
    Ok(())
}
