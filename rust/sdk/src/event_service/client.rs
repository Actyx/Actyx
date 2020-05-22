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
use crate::event::*;
use crate::event_service::*;
use bytes::Bytes;
use futures::future::ready;
use futures::stream::{iter, Stream, StreamExt};
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
/// `http://localhost:4454/api/v1/events/`).
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
        let bytes = response.bytes().await?;
        Ok(serde_json::from_slice(bytes.as_ref())?)
    }

    /// Obtain the local ActyxOS node ID to use it as a source ID in
    /// [`Subscription::local`](struct.Subscription.html#method.local).
    pub async fn node_id(&self) -> Result<SourceId, EventServiceError> {
        let response = self.do_request(|c| c.get(self.url("node_id"))).await?;
        let bytes = response.bytes().await?;
        Ok(serde_json::from_slice::<NodeIdResponse>(bytes.as_ref())?.node_id)
    }

    /// Request a stream of events from the beginning of time until the given upper
    /// bound that must be less than or equal to the currently returned result of
    /// [`get_offsets`](#method.get_offsets) (using that result here is quite common).
    ///
    /// The order of events is specified independently, i.e. if you ask for
    /// [`LamportReverse`](enum.Order.html#variant.LamportReverse) order you’ll
    /// get the events starting with `upper_bound` and
    /// going backwards down to the beginning of time.
    ///
    /// The delivered event stream will be filtered by the subscriptions: an event
    /// is included if any of the subscriptions matches.
    pub async fn query_upto(
        &self,
        upper_bound: OffsetMap,
        subscriptions: Vec<Subscription>,
        order: Order,
    ) -> Result<impl Stream<Item = Event<Payload>>, EventServiceError> {
        let request = QueryApiRequest {
            lower_bound: None,
            upper_bound,
            subscriptions,
            order,
        };
        let body = serde_json::to_value(request)?;
        let response = self
            .do_request(|c| c.post(self.url("query")).json(&body))
            .await?;
        Ok(to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            .filter_map(|res| ready(res.ok())))
    }

    /// Request a stream of events from the given lower bound until the given upper
    /// bound that must be less than or equal to the currently returned result of
    /// [`get_offsets`](#method.get_offsets) (using that result here is quite common).
    ///
    /// The order of events is specified independently, i.e. if you ask for
    /// [`LamportReverse`](enum.Order.html#variant.LamportReverse) order you’ll
    /// get the events starting with `upper_bound` and
    /// going backwards down to the lower bound.
    ///
    /// The delivered event stream will be filtered by the subscriptions: an event
    /// is included if any of the subscriptions matches.
    pub async fn query_between(
        &self,
        lower_bound: OffsetMap,
        upper_bound: OffsetMap,
        subscriptions: Vec<Subscription>,
        order: Order,
    ) -> Result<impl Stream<Item = Event<Payload>>, EventServiceError> {
        let request = QueryApiRequest {
            lower_bound: Some(lower_bound),
            upper_bound,
            subscriptions,
            order,
        };
        let body = serde_json::to_value(request)?;
        let response = self
            .do_request(|c| c.post(self.url("query")).json(&body))
            .await?;
        Ok(to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            .filter_map(|res| ready(res.ok())))
    }

    /// Subscribe to an unbounded stream of events starting at the beginning of
    /// time and continuing past the currently known events (see
    /// [`get_offsets`](#method.get_offsets)) into live mode.
    ///
    /// The common pattern is to take note of consumed events by adding them into an
    /// [`OffsetMap`](../event/struct.OffsetMap.html) and resuming the stream from this
    /// `OffsetMap` after an app restart using [`subscribe_from`](#method.subscribe_from).
    ///
    /// The delivered event stream will be filtered by the subscriptions: an event
    /// is included if any of the subscriptions matches.
    pub async fn subscribe(
        &self,
        subscriptions: Vec<Subscription>,
    ) -> Result<impl Stream<Item = Event<Payload>>, EventServiceError> {
        let request = SubscribeApiRequest {
            lower_bound: None,
            subscriptions,
        };
        let body = serde_json::to_value(request)?;
        let response = self
            .do_request(|c| c.post(self.url("subscribe")).json(&body))
            .await?;
        Ok(to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            .filter_map(|res| ready(res.ok())))
    }

    /// Subscribe to an unbounded stream of events starting at the given lower bound
    /// and continuing past the currently known events (see
    /// [`get_offsets`](#method.get_offsets)) into live mode.
    ///
    /// The common pattern is to take note of consumed events by adding them into an
    /// [`OffsetMap`](../event/struct.OffsetMap.html) and resuming the stream from this
    /// `OffsetMap` after an app restart.
    ///
    /// The delivered event stream will be filtered by the subscriptions: an event
    /// is included if any of the subscriptions matches.
    pub async fn subscribe_from(
        &self,
        lower_bound: OffsetMap,
        subscriptions: Vec<Subscription>,
    ) -> Result<impl Stream<Item = Event<Payload>>, EventServiceError> {
        let request = SubscribeApiRequest {
            lower_bound: Some(lower_bound),
            subscriptions,
        };
        let body = serde_json::to_value(request)?;
        let response = self
            .do_request(|c| c.post(self.url("subscribe")).json(&body))
            .await?;
        Ok(to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            .filter_map(|res| ready(res.ok())))
    }

    /// Publish the given sequence of event payloads in that order in the stream
    /// identified by the given semantics and name. The ActyxOS node will automatically
    /// add the [local source ID](#method.node_id) to mark the origin.
    pub async fn publish<T>(
        &self,
        semantics: Semantics,
        name: FishName,
        events: impl IntoIterator<Item = T>,
    ) -> Result<(), EventServiceError>
    where
        T: Serialize,
    {
        let data: Result<Vec<PublishEvent>, serde_cbor::Error> =
            events.into_iter().try_fold(Vec::new(), |mut v, e| {
                v.push(PublishEvent {
                    semantics: semantics.clone(),
                    name: name.clone(),
                    payload: Payload::compact(&e)?,
                });
                Ok(v)
            });
        let body = serde_json::to_value(PublishRequestBody { data: data? })?;
        self.do_request(|c| c.post(self.url("publish")).json(&body))
            .await?;
        Ok(())
    }

    fn url(&self, path: &str) -> Url {
        self.url.join(path).unwrap()
    }

    async fn do_request(
        &self,
        f: impl FnOnce(&Client) -> RequestBuilder,
    ) -> Result<Response, EventServiceError> {
        let response = f(&self.client).send().await?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let error_code = response.status().as_u16();
            Err(EventServiceError {
                error: response.text().await?,
                error_code,
            })
        }
    }
}

fn to_lines(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>>,
) -> impl Stream<Item = Vec<u8>> {
    let mut buf = Vec::<u8>::new();
    let to_lines = move |bytes: Bytes| {
        buf.extend_from_slice(bytes.as_ref());
        let mut ret = buf
            .split(|b| *b == b'\n')
            .map(|bs| bs.to_vec())
            .collect::<Vec<_>>();
        if let Some(last) = ret.pop() {
            buf.clear();
            buf.extend_from_slice(last.as_ref());
        }
        iter(ret.into_iter().map(|mut bs| {
            if bs.ends_with(b"\r") {
                bs.pop();
            }
            bs
        }))
    };
    stream
        .take_while(|res| ready(res.is_ok()))
        .map(|res| res.unwrap())
        .map(to_lines)
        .flatten()
}

impl Default for EventService {
    /// This will configure a connection to the local Event Service, either an ActyxOS node in development
    /// mode or the production ActyxOS node where the app is deployed (in particular, it will
    /// inspect the `AX_EVENT_SERVICE_URI` environment variable and fall back to
    /// `http://localhost:4454/api`).
    fn default() -> Self {
        let client = Client::new();
        let url = env::var("AX_EVENT_SERVICE_URI")
            .and_then(|uri| Url::parse(&*uri).map_err(|_| env::VarError::NotPresent))
            .unwrap_or_else(|_| Url::parse("http://localhost:4454/api").unwrap())
            .join("/v1/events/")
            .unwrap();
        EventService { client, url }
    }
}

impl From<reqwest::Error> for EventServiceError {
    fn from(e: reqwest::Error) -> Self {
        Self {
            error: format!("{:?}", e),
            error_code: 101,
        }
    }
}

impl From<serde_json::Error> for EventServiceError {
    fn from(e: serde_json::Error) -> Self {
        Self {
            error: format!("{:?}", e),
            error_code: 102,
        }
    }
}

impl From<serde_cbor::Error> for EventServiceError {
    fn from(e: serde_cbor::Error) -> Self {
        Self {
            error: format!("{:?}", e),
            error_code: 102,
        }
    }
}
