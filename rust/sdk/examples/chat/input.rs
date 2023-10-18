use crate::{cmdline::Cmdline, display::Display};
use acto::{ActoCell, ActoRef, ActoRuntime};
use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use void::Void;

pub async fn input(_cell: ActoCell<Void, impl ActoRuntime>, cmdline: ActoRef<Cmdline>, display: ActoRef<Display>) {
    let mut events = EventStream::new();

    while let Some(event) = events.next().await {
        let event = match event {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("error reading event: {}", e);
                return;
            }
        };

        match event {
            Event::Key(key) => {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    tracing::info!("shutting down");
                    return;
                } else {
                    tracing::debug!("sending event: {:?}", key);
                    cmdline.send(Cmdline::Event(Event::Key(key)));
                }
            }
            Event::Resize(_, _) => {
                tracing::debug!("sending event: {:?}", event);
                display.send(Display::Redraw);
            }
            x => tracing::debug!("ignoring event: {:?}", x),
        }
    }
}
