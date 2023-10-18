use actyx_sdk::{app_id, service::PublishEvent, tags, ActyxClient, AppManifest, Payload};
use futures::{stream, FutureExt, StreamExt, TryStreamExt};
use url::Url;

async fn mk_http_client() -> anyhow::Result<ActyxClient> {
    let app_manifest = AppManifest::new(
        app_id!("com.example.actyx-publish"),
        "Publish Example".into(),
        "0.1.0".into(),
        None,
    );
    let url = Url::parse("http://localhost:4454").unwrap();
    ActyxClient::new(url, app_manifest).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = mk_http_client().await?;
    let mut results = stream::iter(0..10).flat_map(|i| {
        let publish_response = service.publish().events(
            vec![
                PublishEvent {
                    tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor1"),
                    payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                },
                PublishEvent {
                    tags: tags!("com.actyx.examples.temperature", "sensor:temp-sensor2"),
                    payload: Payload::compact(&serde_json::json!({ "counter": i })).unwrap(),
                },
            ]
            .into_iter(),
        );
        publish_response.into_stream()
    });

    while let Some(res) = results.try_next().await? {
        println!("{}", serde_json::to_value(res)?);
    }
    Ok(())
}
