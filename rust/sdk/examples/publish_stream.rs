use actyx_sdk::{tags, Ax, AxOpts};
use futures::{stream, FutureExt, StreamExt, TryStreamExt};
use rand::Rng;

// This example demonstrates how to publish values from a stream,
// converting them into events.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup a default Ax client with default settings
    let service = Ax::new(AxOpts::default()).await?;

    // Setup a stream - only generating 10 values so it stops
    let mut results = stream::iter(0..10)
        // Discarding the original value to get a "random temperature"
        .map(|_| rand::thread_rng().gen_range(10..21))
        .flat_map(|i| {
            service
                .publish()
                .event(
                    tags!("temperature", "sensor:temp-sensor"),
                    &serde_json::json!({ "counter": i }),
                )
                .unwrap() // Unwrapping just because we know the payload works
                .into_stream()
        });

    // Consume responses from the subscription stream
    while let Some(res) = results.try_next().await? {
        println!("{}", serde_json::to_value(res)?);
    }

    Ok(())
}
