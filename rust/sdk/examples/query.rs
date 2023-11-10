use actyx_sdk::{
    service::{EventService, Order, QueryRequest, QueryResponse},
    Ax, AxOpts,
};
use futures::stream::StreamExt;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let service = Ax::new(AxOpts::default()).await?;

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
