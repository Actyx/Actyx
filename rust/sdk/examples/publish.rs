use actyxos_sdk::{
    service::{EventService, PublishEvent, PublishRequest},
    tags, HttpClient, Payload,
};
use futures::{stream, FutureExt, Stream, StreamExt, TryStreamExt};

fn counter() -> impl Stream<Item = i32> {
    stream::iter(0..).then(|i| futures_timer::Delay::new(std::time::Duration::from_secs(1)).map(move |()| i))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = HttpClient::default();
    let mut results = counter().flat_map(|i| {
        service
            .publish(PublishRequest {
                data: vec![
                    PublishEvent {
                        tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor1"),
                        payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                    },
                    PublishEvent {
                        tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor2"),
                        payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                    },
                ],
            })
            .into_stream()
    });

    while let Some(res) = results.try_next().await? {
        println!("{}", serde_json::to_value(res)?);
    }
    Ok(())
}
