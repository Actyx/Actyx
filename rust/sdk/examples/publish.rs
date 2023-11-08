use actyx_sdk::{tags, Ax, AxOpts};

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

    // Print the response
    println!("{:?}", publish_response);
    Ok(())
}
