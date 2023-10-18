use actyx_sdk::{app_id, service::QueryResponse, ActyxClient, AppManifest};
use futures::stream::StreamExt;
use url::Url;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let service = mk_http_client().await?;
    let mut events = service.query("FROM 'sensor:temp-sensor1'").await?;
    while let Some(QueryResponse::Event(event)) = events.next().await {
        println!("{}", event.payload.json_value());
    }
    Ok(())
}
