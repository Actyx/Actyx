use actyx_sdk::{
    app_id,
    service::{EventService, Order, QueryRequest, QueryResponse},
    AppManifest, HttpClient,
};
use futures::stream::StreamExt;
use url::Url;

async fn mk_http_client() -> anyhow::Result<HttpClient> {
    let app_manifest = AppManifest::new(
        app_id!("com.example.actyx-offsets"),
        "Offsets Example".into(),
        "0.1.0".into(),
        None,
    );
    let url = Url::parse("http://localhost:4454").unwrap();
    HttpClient::new(url, app_manifest).await
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let service = mk_http_client().await?;

    // all events matching the given subscription
    // sorted backwards, i.e. youngest to oldest
    let request = QueryRequest {
        lower_bound: None,
        upper_bound: None,
        query: "FROM 'sensor:temp-sensor1'".to_owned(),
        order: Order::Desc,
    };
    let mut events = service.query(request).await?;

    while let Some(QueryResponse::Event(event)) = events.next().await {
        println!("{}", event.payload.json_value());
    }
    Ok(())
}
