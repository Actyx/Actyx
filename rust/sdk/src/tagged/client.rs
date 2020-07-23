/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use super::{service::NodeIdResponse, NodeId};
use crate::event_service::client::WithContext;
use crate::{EventServiceError, OffsetMap};
use reqwest::{Client, RequestBuilder, Response};
use std::env;
use url::Url;

/// An Event Service API client with which you can perform queries and publish events.
///
/// This feature is only available under the `client` feature flag.
///
/// The common way to create an EventService instance is to use the default constructor:
///
/// ```rust
/// use actyxos_sdk::event_service::EventService;
///
/// let event_service: EventService = EventService::default();
/// ```
///
/// This will connect to the local Event Service, either an ActyxOS node in development
/// mode or the production ActyxOS node where the app is deployed (in particular, it will
/// inspect the `AX_EVENT_SERVICE_URI` environment variable and fall back to
/// `http://localhost:4454/api/`).
pub struct EventService {
    client: Client,
    url: Url,
}

impl EventService {
    /// Construct a new client from a reqwest [`Client`](https://docs.rs/reqwest/0.10.4/reqwest/struct.Client.html)
    /// and a base URL. The URL _must_ end with a slash as the endpoints below it are resolved as relative paths.
    pub fn new(client: &Client, url: Url) -> Self {
        EventService {
            client: client.clone(),
            url,
        }
    }

    /// Obtain an [`OffsetMap`](../event/struct.OffsetMap.html) that describes the set of all events currently known to the
    /// Event Service. New events are continuously ingested from other ActyxOS nodes, which
    /// means that calling this method again at a later time is likely to produce a larger
    /// `OffsetMap`.
    pub async fn get_offsets(&self) -> Result<OffsetMap, EventServiceError> {
        let response = self.do_request(|c| c.get(self.url("offsets"))).await?;
        let bytes = response
            .bytes()
            .await
            .context(|| format!("getting body for GET {}", self.url("offsets")))?;
        Ok(serde_json::from_slice(bytes.as_ref()).context(|| {
            format!(
                "deserializing offsets from {:?} received from GET {}",
                bytes,
                self.url("offsets")
            )
        })?)
    }

    /// Obtain the local ActyxOS node ID to use it as a source ID in
    /// [`Subscription::local`](struct.Subscription.html#method.local).
    pub async fn node_id(&self) -> Result<NodeId, EventServiceError> {
        let response = self.do_request(|c| c.get(self.url("node_id"))).await?;
        let bytes = response
            .bytes()
            .await
            .context(|| format!("getting body for GET {}", self.url("offsets")))?;
        Ok(serde_json::from_slice::<NodeIdResponse>(bytes.as_ref())
            .context(|| {
                format!(
                    "deserializing node_id from {:?} received from GET {}",
                    bytes,
                    self.url("offsets")
                )
            })?
            .node_id)
    }

    fn url(&self, path: &str) -> Url {
        self.url.join(path).unwrap()
    }

    async fn do_request(
        &self,
        f: impl Fn(&Client) -> RequestBuilder,
    ) -> Result<Response, EventServiceError> {
        let response = f(&self.client)
            .send()
            .await
            .context(|| format!("sending {:?}", f(&self.client)))?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let error_code = response.status().as_u16();
            Err(EventServiceError {
                error: response.text().await.context(|| {
                    format!(
                        "getting body for {} reply to {:?}",
                        error_code,
                        f(&self.client)
                    )
                })?,
                error_code,
                context: format!("sending {:?}", f(&self.client)),
            })
        }
    }
}

impl Default for EventService {
    /// This will configure a connection to the local Event Service, either an ActyxOS node in development
    /// mode or the production ActyxOS node where the app is deployed (in particular, it will
    /// inspect the `AX_EVENT_SERVICE_URI` environment variable and fall back to
    /// `http://localhost:4454/api/`).
    fn default() -> Self {
        let client = Client::new();
        let url = env::var("AX_EVENT_SERVICE_URI")
            .and_then(|mut uri| {
                if !uri.ends_with('/') {
                    uri.push('/')
                };
                Url::parse(&*uri).map_err(|_| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| Url::parse("http://localhost:4454/api/").unwrap())
            .join("v2/events/")
            .unwrap();
        EventService { client, url }
    }
}
