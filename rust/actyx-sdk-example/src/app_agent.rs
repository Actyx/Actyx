use actyx_sdk::{
    self,
    language::Query,
    service::{EventService, PublishEvent, PublishRequest, SessionId, SubscribeMonotonicResponse},
    tag, tags, OffsetMap, Payload,
};
use futures::{future::BoxFuture, pin_mut, stream::PollImmediate, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{pin::Pin, sync::Arc, time::Duration, vec};

#[derive(Serialize, Deserialize)]
pub struct Message {
    from: String,
    message: String,
}

pub enum AppAgentCommands {
    Publish(Message),
}

#[derive(Clone)]
pub struct AppAgent {
    pub identity: String,
    pub is_alive: Arc<tokio::sync::RwLock<bool>>,
    pub client_healthy: Arc<tokio::sync::RwLock<bool>>,
    pub sender: tokio::sync::mpsc::UnboundedSender<AppAgentCommands>,
    pub messages: Arc<tokio::sync::RwLock<Vec<Message>>>,
}

impl AppAgent {
    fn new(sender: tokio::sync::mpsc::UnboundedSender<AppAgentCommands>) -> Self {
        Self {
            identity: rnglib::RNG::random().name,
            is_alive: Arc::new(tokio::sync::RwLock::new(true)),
            client_healthy: Arc::new(tokio::sync::RwLock::new(true)),
            messages: Default::default(),
            sender,
        }
    }

    pub fn kill(&self) -> BoxFuture<()> {
        Box::pin(async {
            *self.is_alive.write().await = false;
        })
    }
}

pub fn init() -> (tokio::task::JoinHandle<()>, AppAgent) {
    use actyx_sdk::{app_id, AppManifest, HttpClient};
    use url::Url;

    let app_manifest = AppManifest::new(
        app_id!("com.example.my-chat-app"),
        "my-chat-app".into(),
        "0.1.0".into(),
        None,
    );

    let url = Url::parse("http://localhost:4454").unwrap();

    let tag = tag!("chat");
    let query: Query = format!("FROM '{}'", &tag).parse().unwrap();
    let tags = tags!(tag);

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<AppAgentCommands>();
    let agent_api = AppAgent::new(sender);
    let agent_api_external = agent_api.clone();

    // worker

    let agent_handle = tokio::spawn(async move {
        let identity = agent_api.identity.clone();
        let client_healthy = agent_api.client_healthy.clone();
        let is_alive = agent_api.is_alive.clone();
        let messages = agent_api.messages.clone();

        let service_res = HttpClient::new(url, app_manifest).await;
        let service = match service_res {
            Err(error) => {
                let mut health_flag = client_healthy.write().await;
                *health_flag = false;
                eprintln!("{:?}", error);
                return;
            }
            Ok(x) => x,
        };

        let subscription = match service
            .subscribe_monotonic(actyx_sdk::service::SubscribeMonotonicRequest {
                query: query.clone(),
                session: SessionId::from(identity),
                from: actyx_sdk::service::StartFrom::LowerBound(OffsetMap::empty()),
            })
            .await
        {
            Err(error) => {
                let mut health_flag = client_healthy.write().await;
                *health_flag = false;
                eprintln!("{:?}", error);
                return;
            }
            Ok(x) => x,
        };

        let mut poll_immediate = futures::stream::poll_immediate(subscription);

        loop {
            let alive = {
                let is_alive = *is_alive.read().await;
                let is_healthy = *client_healthy.read().await;

                is_alive && is_healthy
            };
            if !alive {
                break;
            }

            if let GatherResponse::Change {
                payloads,
                should_clear,
            } = GatherResponse::create(&mut poll_immediate).await
            {
                let mut messages = messages.write().await;
                if should_clear {
                    messages.clear();
                }

                messages.extend(
                    payloads
                        .into_iter()
                        .filter_map(|x| serde_json::from_value::<Message>(x.json_value()).ok()),
                )
            }

            // Send buffered messages

            match receiver.try_recv() {
                Ok(x) => match x {
                    AppAgentCommands::Publish(message) => {
                        let value = if let Ok(value) = serde_json::to_value(message) {
                            value
                        } else {
                            return;
                        };
                        let payload = if let Ok(payload) = Payload::from_json_value(value) {
                            payload
                        } else {
                            return;
                        };
                        let _ = service
                            .publish(PublishRequest {
                                data: vec![PublishEvent {
                                    tags: tags.clone(),
                                    payload,
                                }],
                            })
                            .await;
                    }
                },
                Err(_) => {}
            };

            // TODO: lower sleep time
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });

    (agent_handle, agent_api_external)
}

enum GatherResponse {
    Change {
        payloads: Vec<Payload>,
        should_clear: bool,
    },
    NoChange,
}

impl GatherResponse {
    pub async fn create(
        message_poll: &mut PollImmediate<
            Pin<Box<dyn Stream<Item = SubscribeMonotonicResponse> + Send>>,
        >,
    ) -> Self {
        let mut should_clear = false;
        let mut payloads: Vec<Payload> = Default::default();

        while let Some(std::task::Poll::Ready(response)) = message_poll.next().await {
            match response {
                SubscribeMonotonicResponse::Event {
                    event,
                    caught_up: _,
                } => payloads.push(event.payload),
                SubscribeMonotonicResponse::Offsets(_) => {}
                SubscribeMonotonicResponse::TimeTravel { new_start: _ } => {
                    payloads.clear();
                    should_clear = true;
                }
                SubscribeMonotonicResponse::Diagnostic(_) => {}
                SubscribeMonotonicResponse::FutureCompat => {}
            };
        }

        if should_clear || payloads.len() > 0 {
            GatherResponse::Change {
                payloads,
                should_clear,
            }
        } else {
            GatherResponse::NoChange
        }
    }
}
