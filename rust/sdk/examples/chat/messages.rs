use crate::display::Display;
use acto::{variable::Writer, ActoCell, ActoInput, ActoRef, ActoRuntime};
use actyx_sdk::{
    app_id,
    service::{
        EventService, PublishEvent, PublishRequest, SessionId, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse,
    },
    tags, ActyxClient, AppManifest, OffsetMap, Payload, Timestamp,
};
use futures::{
    future::{select, Either},
    StreamExt,
};
use serde::{Deserialize, Serialize};

pub struct Message {
    pub time: Timestamp,
    pub from: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    from: String,
    text: String,
}

impl Event {
    pub fn new(from: String, text: String) -> Self {
        Self { from, text }
    }
}

pub enum Messages {
    Publish(Event),
}

pub async fn messages(mut cell: ActoCell<Messages, impl ActoRuntime>, display: ActoRef<Display>) {
    // FIXME too verbose
    let client = ActyxClient::new(
        "http://localhost:4454".parse().unwrap(),
        AppManifest::new(
            app_id!("com.example.chat"),
            "simple chat".to_owned(),
            "0.1.0".to_owned(),
            None,
        ),
    )
    .await;
    let client = match client {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to connect to Actyx: {}", e);
            return;
        }
    };

    let mk_sub = || async {
        // FIXME too verbose
        client
            .subscribe_monotonic(SubscribeMonotonicRequest {
                session: SessionId::from("me"),
                query: "FROM appId(com.example.chat) & 'message'".to_owned(),
                from: StartFrom::LowerBound(OffsetMap::empty()),
            })
            .await
            .expect("failed to subscribe to messages")
    };

    let messages = Writer::new(Vec::<Message>::new());
    let mut sub = mk_sub().await;

    loop {
        match select(cell.recv(), sub.next()).await {
            Either::Left((input, _)) => {
                if let ActoInput::Message(msg) = input {
                    match msg {
                        Messages::Publish(event) => {
                            // FIXME too verbose
                            client
                                .publish(PublishRequest {
                                    data: vec![PublishEvent {
                                        tags: tags!("message"),
                                        payload: Payload::compact(&event).expect("failed to serialize event"),
                                    }],
                                })
                                .await
                                .expect("failed to publish event");
                        }
                    }
                } else {
                    tracing::info!("messages stopped via ActoRef");
                    return;
                }
            }
            Either::Right((resp, _)) => {
                if let Some(resp) = resp {
                    tracing::debug!("received response: {:?}", resp);
                    match resp {
                        SubscribeMonotonicResponse::Event { event, caught_up } => {
                            if let Ok(p) = event.payload.extract::<Event>() {
                                messages.write().push(Message {
                                    time: event.meta.left().1,
                                    from: p.from,
                                    text: p.text,
                                });
                            }
                            if caught_up {
                                display.send(Display::Messages(messages.reader()));
                            }
                        }
                        SubscribeMonotonicResponse::TimeTravel { .. } => {
                            messages.write().clear();
                            sub = mk_sub().await;
                        }
                        SubscribeMonotonicResponse::Offsets(_) => {
                            // FIXME waiting for caught_up should somehow suffice
                            display.send(Display::Messages(messages.reader()));
                        }
                        SubscribeMonotonicResponse::Diagnostic(_) | SubscribeMonotonicResponse::FutureCompat => {}
                    }
                } else {
                    tracing::info!("messages stopped via subscription");
                    return;
                }
            }
        }
    }
}
