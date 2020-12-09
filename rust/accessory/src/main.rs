// Hide command prompt
#![windows_subsystem = "windows"]

use accessory::card_poll_loop;
use actyxos_sdk::{event::FishName, event_service::EventService, fish_name, semantics};
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio_compat_02::FutureExt;
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let (tx, mut rx) = mpsc::channel(256);
    let event_service = EventService::default();

    let _h = std::thread::spawn(|| {
        card_poll_loop(Some(tx)).unwrap();
    });

    let semantics = semantics!("cardReader");
    while let Some(ev) = rx.recv().await {
        println!("{:?}", ev);
        let name: FishName = FishName::from_str(&ev.reader.friendly_name).unwrap_or(fish_name!("Unknown"));
        if let Err(e) = event_service
            .publish(semantics.clone(), name, std::iter::once(ev))
            .compat()
            .await
        {
            error!("Error trying to publish event {}. Discarding event.", e);
        }
    }

    Ok(())
}
