use actyxos_sdk::event_service::{EventService, EventServiceError, Order, Subscription};
use futures::stream::StreamExt;

#[tokio::main]
pub async fn main() -> Result<(), EventServiceError> {
    let service = EventService::default();
    println!("source ID is {}", service.node_id().await?);
    let offsets = service.get_offsets().await?;
    let mut events = service
        .query_upto(
            offsets,
            vec![Subscription::semantics("edge.ax.sf.Terminal".into())],
            Order::LamportReverse,
        )
        .await?;
    while let Some(event) = events.next().await {
        println!("{}", event.payload.json_value());
    }
    Ok(())
}
