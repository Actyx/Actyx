use anyhow::Result;
use bytes::Bytes;
use derive_more::{Display, Error};
use futures::{
    future::{self, BoxFuture, FusedFuture},
    stream::{iter, BoxStream, Stream, StreamExt},
    FutureExt,
};

use libipld::Cid;
use reqwest::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    multipart::Form,
    Client, RequestBuilder, Response, StatusCode,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    future::Future,
    mem::replace,
    pin::Pin,
    str::FromStr,
    sync::{Arc, RwLock},
};
use url::Url;

#[cfg(feature = "with-tokio")]
use rand::Rng;
#[cfg(feature = "with-tokio")]
use std::time::Duration;

use crate::{
    service::{
        AuthenticationResponse, FilesGetResponse, OffsetsResponse, Order, PublishEvent, PublishRequest,
        PublishResponse, QueryRequest, QueryResponse, SessionId, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    AppManifest, NodeId, OffsetMap, Payload, TagSet,
};

/// [`Ax`]'s configuration options.
pub struct AxOpts {
    pub url: url::Url,
    pub manifest: AppManifest,
}

impl AxOpts {
    /// Create an [`AxOpts`] with a custom URL and the default application manifest.
    ///
    /// This function is similar manually constructing the following:
    /// ```no_run
    /// # use actyx_sdk::AxOpts;
    /// # fn opts() -> AxOpts {
    /// AxOpts {
    ///     url: "https://your.host:1234".parse().unwrap(),
    ///     ..Default::default()
    /// }.into()
    /// # }
    /// ```
    pub fn url(url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            url: Url::from_str(url)?,
            ..Default::default()
        })
    }

    /// Create an [`AxOpts`] with a custom application manifest and the default URL.
    ///
    /// This function is similar manually constructing the following:
    /// ```no_run
    /// # use actyx_sdk::{app_id, AppManifest, AxOpts};
    /// # fn opts() -> AxOpts {
    /// AxOpts {
    ///     manifest: AppManifest {
    ///         app_id: app_id!("com.example.app"),
    ///         display_name: "Example manifest".to_string(),
    ///         version: "0.1.0".to_string(),
    ///         signature: None,
    ///     },
    ///     ..Default::default()
    /// }.into()
    /// # }
    /// ```
    pub fn manifest(manifest: AppManifest) -> anyhow::Result<Self> {
        Ok(Self {
            manifest,
            ..Default::default()
        })
    }
}

impl Default for AxOpts {
    /// Return a default set of options.
    ///
    /// The default URL is `http://localhost:4454`,
    /// for the default manifest see [`AppManifest`].
    fn default() -> Self {
        Self {
            url: url::Url::from_str("http://localhost:4454").unwrap(),
            manifest: Default::default(),
        }
    }
}

async fn get_token(client: &Client, base_url: &Url, app_manifest: &AppManifest) -> anyhow::Result<String> {
    let body = serde_json::to_value(app_manifest).context(|| format!("serializing {:?}", app_manifest))?;
    let response = client.post(base_url.join("auth")?).json(&body).send().await?;
    let bytes = response
        .bytes()
        .await
        .context(|| "getting body for authentication response")?;
    let token: AuthenticationResponse =
        serde_json::from_slice(bytes.as_ref()).context(|| "deserializing authentication response")?;
    Ok(token.token)
}

/// The Actyx client.
#[derive(Clone)]
pub struct Ax {
    client: Client,
    base_url: Url,
    token: Arc<RwLock<String>>,
    app_manifest: AppManifest,
    node_id: NodeId,
}

impl Ax {
    /// Instantiate a new [`Ax`] with the provided options.
    ///
    /// See [`AxOpts`] for more information.
    pub async fn new(opts: AxOpts) -> anyhow::Result<Self> {
        let origin = opts.url;
        let app_manifest = opts.manifest;

        // NOTE(duarte): we could probably validate this in the opts
        // We would need to provide a `new` instead of letting users do struct instantiation by hand though
        anyhow::ensure!(!origin.cannot_be_a_base(), "{} is not a valid base address", origin);
        let mut base_url = origin;
        base_url.set_path("api/v2/");
        let client = Client::new();

        let node_id = client
            .get(base_url.join("node/id").unwrap())
            .send()
            .await?
            .text()
            .await
            .context(|| "getting body for GET node/id")?
            .parse()?;

        let token = get_token(&client, &base_url, &app_manifest).await?;

        Ok(Self {
            client,
            base_url,
            token: Arc::new(RwLock::new(token)),
            app_manifest,
            node_id,
        })
    }

    /// Return the ID of the node [`Ax`] is connected to.
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub(crate) fn events_url(&self, path: &str) -> Url {
        // Safe to unwrap, because we fully control path creation
        self.base_url.join(&format!("events/{}", path)).unwrap()
    }

    pub(crate) fn files_url(&self) -> Url {
        self.base_url.join("files/").unwrap()
    }

    async fn re_authenticate(&self) -> anyhow::Result<String> {
        let token = get_token(&self.client, &self.base_url, &self.app_manifest).await?;
        let mut write_guard = self.token.write().unwrap();
        *write_guard = token.clone();
        Ok(token)
    }

    /// Perform a request (to Actyx APIs).
    /// If an authorization error (code 401) is returned, it will try to re-authenticate.
    /// If the service is unavailable (code 503), this method will retry to perform the
    /// request up to 10 times with exponentially increasing delay - currently,
    /// this behavior is only available if the `with-tokio` feature is enabled.
    pub(crate) async fn do_request(&self, f: impl FnOnce(&Client) -> RequestBuilder) -> anyhow::Result<Response> {
        let token = self.token.read().unwrap().clone();
        let builder = f(&self.client);
        let builder_clone = builder.try_clone();

        let req = builder.header("Authorization", &format!("Bearer {}", token)).build()?;
        let url = req.url().clone();
        let method = req.method().clone();
        let mut response = self
            .client
            .execute(req)
            .await
            .context(|| format!("sending {} {}", method, url))?;

        if let Some(builder) = builder_clone.as_ref() {
            // Request body is not a Stream, so we can retry
            if response.status() == StatusCode::UNAUTHORIZED {
                let token = self.re_authenticate().await?;
                response = builder
                    .try_clone()
                    .expect("Already cloned it once")
                    .header("Authorization", &format!("Bearer {}", token))
                    .send()
                    .await
                    .context(|| format!("sending {} {}", method, url))?;
            }

            #[cfg(feature = "with-tokio")]
            {
                let mut retries = 10;
                let mut delay = Duration::from_secs(0);
                loop {
                    if response.status() == StatusCode::SERVICE_UNAVAILABLE && retries > 0 {
                        retries -= 1;
                        delay = delay * 2 + Duration::from_millis(rand::thread_rng().gen_range(10..200));
                        tracing::debug!(
                            "Actyx Node is overloaded, retrying {} {} with a delay of {:?}",
                            method,
                            url,
                            delay
                        );
                        tokio::time::sleep(delay).await;
                        response = builder
                            .try_clone()
                            .expect("Already cloned it once")
                            .header("Authorization", &format!("Bearer {}", token))
                            .send()
                            .await
                            .context(|| format!("sending {} {}", method, url))?;
                    } else {
                        break;
                    }
                }
            }
        } else {
            tracing::warn!("Request can't be retried, as its body is based on a stream");
            // Request body is a stream, so impossible to retry
            if response.status() == StatusCode::UNAUTHORIZED {
                tracing::info!("Can't retry request, but re-authenticated anyway. SDK user must retry request.");
                self.re_authenticate().await?;
            }
        }

        if response.status().is_success() {
            Ok(response)
        } else {
            let error_code = response.status().as_u16();
            Err(AxError {
                error: response
                    .json()
                    .await
                    .context(|| format!("getting body for {} reply to {:?}", error_code, builder_clone))?,
                error_code,
                context: format!("sending {:?}", builder_clone),
            }
            .into())
        }
    }

    // TODO: #558
    pub async fn files_post(&self, files: impl IntoIterator<Item = reqwest::multipart::Part>) -> anyhow::Result<Cid> {
        let mut form = Form::new();
        for file in files {
            form = form.part("file", file);
        }
        let response = self
            .do_request(move |c| c.post(self.files_url()).multipart(form))
            .await?;
        let hash = response
            .text_with_charset("utf-8")
            .await
            .context(|| "Parsing response".to_string())?;
        let cid = Cid::from_str(&hash).map_err(|e| AxError {
            error: serde_json::Value::String(e.to_string()),
            error_code: 102,
            context: format!("Tried to parse {} into a Cid", hash),
        })?;
        Ok(cid)
    }

    // TODO: #558
    pub async fn files_get(&self, cid_or_name: &str) -> anyhow::Result<FilesGetResponse> {
        let url = self.files_url().join(cid_or_name)?;
        let response = self.do_request(move |c| c.get(url)).await?;

        let maybe_name = response.headers().get(CONTENT_DISPOSITION).cloned();
        let maybe_mime = response.headers().get(CONTENT_TYPE).cloned();
        let bytes = response.bytes().await?;
        if let Ok(dir @ FilesGetResponse::Directory { .. }) = serde_json::from_slice(bytes.as_ref()) {
            Ok(dir)
        } else {
            let mime = maybe_mime
                .and_then(|h| h.to_str().ok().map(|x| x.to_string()))
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let name = maybe_name
                .and_then(|n| {
                    n.to_str().ok().and_then(|p| {
                        p.split(';')
                            .find(|x| x.starts_with("filename="))
                            .map(|f| f.trim_start_matches("filename=").to_string())
                    })
                })
                .unwrap_or_default();
            Ok(FilesGetResponse::File {
                name,
                bytes: bytes.to_vec(),
                mime,
            })
        }
    }

    /// Returns known offsets across local and replicated streams.
    ///
    /// If an authorization error (code 401) is returned, it will try to re-authenticate.
    /// If the service is unavailable (code 503), this method will retry to perform the
    /// request up to 10 times with exponentially increasing delay - currently,
    /// this behavior is only available if the `with-tokio` feature is enabled.
    pub async fn offsets(&self) -> anyhow::Result<OffsetsResponse> {
        let response = self.do_request(|c| c.get(self.events_url("offsets"))).await?;
        let bytes = response
            .bytes()
            .await
            .context(|| format!("getting body for GET {}", self.events_url("offsets")))?;
        Ok(serde_json::from_slice(bytes.as_ref()).context(|| {
            format!(
                "deserializing offsets response from {:?} received from GET {}",
                bytes,
                self.events_url("offsets")
            )
        })?)
    }

    /// Returns a builder for publishing events.
    ///
    /// [`Publish`] implements the [`Future`] trait, thus, it can be `.await`ed.
    ///
    /// Example:
    /// ```no_run
    /// use sdk::{Ax, AxOpts, PublishResponse}
    /// async fn publish_example() {
    ///     let response = Ax::new(AxOpts::default())
    ///         .publish()
    ///         .await
    ///         .unwrap();
    ///     println!("{:?}", response);
    /// }
    /// ```
    pub fn publish(&self) -> Publish<'_> {
        Publish::new(&self)
    }

    /// Returns a builder to query events.
    ///
    /// Query order defined in the query itself takes precedence over options.
    /// See [`Query::with_order`] for more information.
    ///
    /// [`Query`] implements the [`Future`] trait, thus, it can be `.await`ed.
    ///
    /// Example:
    /// ```no_run
    /// use sdk::{Ax, AxOpts, QueryResponse}
    /// async fn query_example() {
    ///     let response = Ax::new(AxOpts::default())
    ///         .query("FROM allEvents")
    ///         .await
    ///         .unwrap();
    ///     println!("{:?}", response);
    /// }
    /// ```
    pub fn query<Q: Into<String> + Send>(&self, query: Q) -> Query<'_> {
        Query::new(&self, query)
    }

    /// Returns a builder to subscribe to an event query.
    ///
    /// [`Subscribe`] implements the [`Future`] trait, thus, it can be `.await`ed.
    ///
    /// Example:
    /// ```no_run
    /// use sdk::{Ax, AxOpts, SubscribeResponse}
    /// async fn subscribe_example() {
    ///     let service = Ax::new(AxOpts::default()).await.unwrap();
    ///     let mut subscribe_stream = service.subscribe("FROM 'example:tag'").await.unwrap();
    ///     while let Some(response) = subscribe_stream.next().await {
    ///         println!("{:?}", response)
    ///     }
    /// }
    /// ```
    pub fn subscribe<Q: Into<String> + Send>(&self, query: Q) -> Subscribe<'_> {
        Subscribe::new(&self, query)
    }

    /// Returns a builder to subscribe to an event query.
    ///
    /// [`SubscribeMonotonic`] implements the [`Future`] trait, thus, it can be `.await`ed.
    ///
    /// Example:
    /// ```no_run
    /// use sdk::{Ax, AxOpts, SubscribeMonotonicResponse}
    /// async fn subscribe_monotonic_example() {
    ///     let service = Ax::new(AxOpts::default()).await.unwrap();
    ///     let mut subscribe_stream = service.subscribe_monotonic("FROM 'example:tag'").await.unwrap();
    ///     while let Some(response) = subscribe_stream.next().await {
    ///         println!("{:?}", response)
    ///     }
    /// }
    /// ```
    pub fn subscribe_monotonic<Q: Into<String> + Send>(&self, query: Q) -> SubscribeMonotonic<'_> {
        SubscribeMonotonic::new(&self, query)
    }
}

impl Debug for ActyxClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActyxClient")
            .field("base_url", &self.base_url.as_str())
            .field("app_manifest", &self.app_manifest)
            .finish()
    }
}

pub(crate) fn to_lines(stream: impl Stream<Item = Result<Bytes, reqwest::Error>>) -> impl Stream<Item = Vec<u8>> {
    let mut buf = Vec::<u8>::new();
    let to_lines = move |bytes: Bytes| {
        buf.extend_from_slice(bytes.as_ref());
        let mut ret = buf.split(|b| *b == b'\n').map(|bs| bs.to_vec()).collect::<Vec<_>>();
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
        .take_while(|res| future::ready(res.is_ok()))
        .map(|res| res.unwrap())
        .map(to_lines)
        .flatten()
}

/// Request builder for event publishing.
///
/// Warning: [`Publish`] implements the [`Future`] trait, as such it can be polled.
/// Calling _any_ [`Publish`] function after polling will result in a panic!
pub enum Publish<'a> {
    Initial {
        client: &'a ActyxClient,
        request: PublishRequest,
    },
    Pending(BoxFuture<'a, anyhow::Result<PublishResponse>>),
    Void,
}

impl<'a> Publish<'a> {
    fn new(client: &'a ActyxClient) -> Self {
        Self::Initial {
            client,
            request: PublishRequest { data: vec![] },
        }
    }

    /// Add an event.
    ///
    /// Subsequent calls to this function will not remove previous events.
    ///
    /// Example:
    /// ```no_run
    /// use sdk::{Ax, AxOpts, PublishResponse};
    /// async fn main() -> PublishResponse {
    ///     let service = Ax::new(AxOpts::default()).await.unwrap();
    ///     service
    ///         .publish()
    ///         .event(
    ///             tags!("temperature", "sensor:temp-sensor1"),
    ///             &serde_json::json!({ "temperature": 10 }),
    ///         )
    ///         .unwrap()
    ///         .event(
    ///             tags!("temperature", "sensor:temp-sensor2"),
    ///             &serde_json::json!({ "temperature": 21 }),
    ///         )
    ///         .unwrap()
    ///         .await
    ///         .unwrap()
    /// }
    /// ```
    pub fn event<E: Serialize>(mut self, tags: TagSet, event: &E) -> Result<Self, serde_cbor::Error> {
        if let Self::Initial { ref mut request, .. } = self {
            request.data.push(PublishEvent {
                tags,
                payload: Payload::compact(event)?,
            });
        }
        Ok(self)
    }

    /// Add events from an iterable.
    ///
    /// Subsequent calls to this function will not remove previous events.
    ///
    /// Example:
    /// ```no_run
    /// use sdk::{Ax, AxOpts, PublishResponse};
    /// async fn main() -> PublishResponse {
    ///     let service = Ax::new(AxOpts::default()).await.unwrap();
    ///     service
    ///         .publish()
    ///         .events([
    ///             PublishEvent {
    ///                 tags: tags!("temperature", "sensor:temp-sensor1"),
    ///                 payload: Payload::compact(&serde_json::json!({ "temperature": 10 })).unwrap(),
    ///             },
    ///             PublishEvent {
    ///                 tags: tags!("temperature", "sensor:temp-sensor2"),
    ///                 payload: Payload::compact(&serde_json::json!({ "temperature": 27 })).unwrap(),
    ///             },
    ///         ])
    ///         .await
    ///         .unwrap()
    /// }
    /// ```
    pub fn events<E: IntoIterator<Item = impl Into<PublishEvent>>>(mut self, events: E) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.data.extend(events.into_iter().map(Into::into));
        }
        self
    }
}

impl<'a> Future for Publish<'a> {
    type Output = anyhow::Result<PublishResponse>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            *this = match replace(this, Publish::Void) {
                Publish::Initial { client, request } => {
                    let publish_response = async move {
                        let publish_url = client.events_url("publish");
                        let response = client.do_request(|c| c.post(publish_url).json(&request)).await?;
                        let body = response.bytes().await?;
                        Ok(serde_json::from_slice::<PublishResponse>(&body)?)
                    };
                    Publish::Pending(publish_response.boxed())
                }
                Publish::Pending(mut publish_response_future) => {
                    let polled = publish_response_future.poll_unpin(cx);
                    if polled.is_pending() {
                        *this = Publish::Pending(publish_response_future);
                    }
                    return polled;
                }
                Publish::Void => panic!("Polling a terminated Publish future"),
            };
        }
    }
}

impl<'a> FusedFuture for Publish<'a> {
    fn is_terminated(&self) -> bool {
        if let Publish::Void = self {
            true
        } else {
            false
        }
    }
}

/// Request builder for queries.
///
/// Warning: [`Query`] implements the [`Future`] trait, as such it can be polled.
/// Calling _any_ [`Query`] function after polling will result in a panic!
pub enum Query<'a> {
    Initial {
        client: &'a ActyxClient,
        request: QueryRequest,
    },
    Pending(BoxFuture<'a, anyhow::Result<BoxStream<'a, QueryResponse>>>),
    Void,
}

impl<'a> Query<'a> {
    fn new<Q: Into<String>>(client: &'a ActyxClient, query: Q) -> Self {
        Self::Initial {
            client,
            request: QueryRequest {
                query: query.into(),
                lower_bound: Some(OffsetMap::empty()),
                upper_bound: None,
                order: Order::Asc,
            },
        }
    }

    /// Add a lower bound to the query.
    ///
    /// The lower bound limits the start of the query events.
    /// As an example, consider the following (example) events:
    /// ```json
    /// { "lamport": 1, "event": { "temperature": 10 } }
    /// { "lamport": 3, "event": { "temperature": 12 } }
    /// { "lamport": 14, "event": { "temperature": 9 } }
    /// ```
    /// If you set the lower bound to `10`, only the last event will be returned.
    pub fn with_lower_bound(mut self, lower_bound: OffsetMap) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.lower_bound = Some(lower_bound);
        }
        panic!("Calling Query::with_lower_bound after polling.")
    }

    /// Add an upper bound to the query.
    ///
    /// The upper bound limits the start of the query events.
    /// As an example, consider the following (example) events:
    /// ```json
    /// { "lamport": 1, "event": { "temperature": 10 } }
    /// { "lamport": 3, "event": { "temperature": 12 } }
    /// { "lamport": 14, "event": { "temperature": 9 } }
    /// ```
    /// If you set the upper bound to `10`, the first two events will be returned.
    pub fn with_upper_bound(mut self, upper_bound: OffsetMap) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.upper_bound = Some(upper_bound);
        }
        panic!("Calling Query::with_upper_bound after polling.")
    }

    /// Dual to [`Query::with_upper_bound`], removes the upper bound.
    pub fn without_upper_bound(mut self) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.upper_bound = None;
        }
        panic!("Calling Query::without_upper_bound after polling.")
    }

    /// Set the query's event order.
    ///
    /// By default, this value is set to [`Order::Asc`], however,
    /// order set in the query takes precedence over the order defined using this function.
    /// The precedence order flows like so:
    /// 1. Explicit `ORDER` in query
    /// 2. Inferred from `AGGREGATE` in query
    /// 3. [`Query::with_order`] call
    ///
    /// As an example, consider the following (example) events:
    /// ```json
    /// { "lamport": 1, "event": { "temperature": 10 } }
    /// { "lamport": 3, "event": { "temperature": 12 } }
    /// { "lamport": 14, "event": { "temperature": 9 } }
    /// ```
    /// If your query sets [`Order::Desc`], the result will instead look like:
    /// ```json
    /// { "lamport": 14, "event": { "temperature": 9 } }
    /// { "lamport": 3, "event": { "temperature": 12 } }
    /// { "lamport": 1, "event": { "temperature": 10 } }
    /// ```
    pub fn with_order(mut self, order: Order) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.order = order;
        }
        panic!("Calling Query::with_order after polling.")
    }
}

impl<'a> Future for Query<'a> {
    type Output = anyhow::Result<BoxStream<'a, QueryResponse>>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            *this = match replace(this, Query::Void) {
                Query::Initial { client, request } => {
                    let query_response = async move {
                        let query_url = client.events_url("query");
                        let response = client.do_request(|c| c.post(query_url).json(&request)).await?;
                        let response_stream = to_lines(response.bytes_stream())
                            .map(|bytes| serde_json::from_slice::<QueryResponse>(&bytes))
                            // FIXME this swallows deserialization errors, silently dropping event envelopes
                            .filter_map(|res| future::ready(res.ok()))
                            .boxed();
                        Ok(response_stream)
                    };
                    Query::Pending(query_response.boxed())
                }
                Query::Pending(mut query_responses_future) => {
                    let polled = query_responses_future.poll_unpin(cx);
                    if polled.is_pending() {
                        *this = Query::Pending(query_responses_future);
                    }
                    return polled;
                }
                Query::Void => panic!("Polling a terminated Query future"),
            }
        }
    }
}

/// Request builder for subscriptions.
///
/// Warning: [`Subscribe`] implements the [`Future`] trait, as such it can be polled.
/// Calling _any_ [`Subscribe`] function after polling will result in a panic!
pub enum Subscribe<'a> {
    Initial {
        client: &'a ActyxClient,
        request: SubscribeRequest,
    },
    Pending(BoxFuture<'a, anyhow::Result<BoxStream<'a, SubscribeResponse>>>),
    Void,
}

impl<'a> Subscribe<'a> {
    fn new<Q: Into<String>>(client: &'a ActyxClient, query: Q) -> Self {
        Self::Initial {
            client,
            request: SubscribeRequest {
                query: query.into(),
                lower_bound: Some(OffsetMap::empty()),
            },
        }
    }

    /// Add a lower bound to the subscription query.
    ///
    /// The lower bound limits the start of the query events.
    /// As an example, consider the following (example) events:
    /// ```json
    /// { "lamport": 1, "event": { "temperature": 10 } }
    /// { "lamport": 3, "event": { "temperature": 12 } }
    /// { "lamport": 14, "event": { "temperature": 9 } }
    /// ```
    /// If you set the lower bound to `10`, the first event to be returned
    /// would be the last of the example.
    pub fn with_lower_bound(mut self, lower_bound: OffsetMap) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.lower_bound = Some(lower_bound);
        }
        panic!("Calling Subscribe::with_lower_bound after polling.")
    }
}

impl<'a> Future for Subscribe<'a> {
    type Output = anyhow::Result<BoxStream<'a, SubscribeResponse>>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            *this = match replace(this, Self::Void) {
                Self::Initial { client, request } => {
                    let query_response = async move {
                        let query_url = client.events_url("subscribe");
                        let response = client.do_request(|c| c.post(query_url).json(&request)).await?;
                        let response_stream = to_lines(response.bytes_stream())
                            .map(|bytes| serde_json::from_slice::<SubscribeResponse>(&bytes))
                            // FIXME this swallows deserialization errors, silently dropping event envelopes
                            .filter_map(|res| future::ready(res.ok()))
                            .boxed();
                        Ok(response_stream)
                    };
                    Self::Pending(query_response.boxed())
                }
                Self::Pending(mut query_responses_future) => {
                    let polled = query_responses_future.poll_unpin(cx);
                    if polled.is_pending() {
                        *this = Self::Pending(query_responses_future);
                    }
                    return polled;
                }
                Self::Void => panic!("Polling a terminated Query future"),
            }
        }
    }
}

/// Request builder for subscriptions.
///
/// Warning: [`SubscribeMonotonic`] implements the [`Future`] trait, as such it can be polled.
/// Calling _any_ [`SubscribeMonotonic`] function after polling will result in a panic!
pub enum SubscribeMonotonic<'a> {
    Initial {
        client: &'a ActyxClient,
        request: SubscribeMonotonicRequest,
    },
    Pending(BoxFuture<'a, anyhow::Result<BoxStream<'a, SubscribeMonotonicResponse>>>),
    Void,
}

impl<'a> SubscribeMonotonic<'a> {
    fn new<Q: Into<String>>(client: &'a ActyxClient, query: Q) -> Self {
        Self::Initial {
            client,
            request: SubscribeMonotonicRequest {
                query: query.into(),
                session: SessionId::from("me"),
                from: StartFrom::LowerBound(OffsetMap::empty()),
            },
        }
    }

    // NOTE: Currently not being used. This is an "artifact" for future reference.
    #[allow(dead_code)]
    fn with_session_id<T: Into<SessionId>>(mut self, session_id: T) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.session = session_id.into();
        }
        panic!("Calling SubscribeMonotonic::with_session_id after polling.")
    }

    // TODO: Figure out how this is different from the lower bound
    pub fn with_start_from(mut self, start_from: StartFrom) -> Self {
        if let Self::Initial { ref mut request, .. } = self {
            request.from = start_from;
        }
        panic!("Calling SubscribeMonotonic::with_start_from after polling.")
    }
}

impl<'a> Future for SubscribeMonotonic<'a> {
    type Output = anyhow::Result<BoxStream<'a, SubscribeMonotonicResponse>>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            *this = match replace(this, Self::Void) {
                Self::Initial { client, request } => {
                    let query_response = async move {
                        let query_url = client.events_url("subscribe_monotonic");
                        let response = client.do_request(|c| c.post(query_url).json(&request)).await?;
                        let response_stream = to_lines(response.bytes_stream())
                            .map(|bytes| serde_json::from_slice::<SubscribeMonotonicResponse>(&bytes))
                            // FIXME this swallows deserialization errors, silently dropping event envelopes
                            .filter_map(|res| future::ready(res.ok()))
                            .boxed();
                        Ok(response_stream)
                    };
                    Self::Pending(query_response.boxed())
                }
                Self::Pending(mut query_responses_future) => {
                    let polled = query_responses_future.poll_unpin(cx);
                    if polled.is_pending() {
                        *this = Self::Pending(query_responses_future);
                    }
                    return polled;
                }
                Self::Void => panic!("Polling a terminated Query future"),
            }
        }
    }
}

/// Error type that is returned in the response body by the Event Service when requests fail
///
/// The Event Service does not map client errors or internal errors to HTTP status codes,
/// instead it gives more structured information using this data type, except when the request
/// is not understood at all.
#[derive(Clone, Debug, Error, Display, Serialize, Deserialize, PartialEq, Eq)]
#[display(fmt = "error {} while {}: {}", error_code, context, error)]
#[serde(rename_all = "camelCase")]
pub struct ActyxClientError {
    pub error: serde_json::Value,
    pub error_code: u16,
    pub context: String,
}

pub(crate) trait WithContext {
    type Output;
    fn context<F, T>(self, context: F) -> Self::Output
    where
        T: Into<String>,
        F: FnOnce() -> T;
}
impl<T, E> WithContext for std::result::Result<T, E>
where
    ActyxClientError: From<(String, E)>,
{
    type Output = std::result::Result<T, ActyxClientError>;

    #[inline]
    fn context<F, C>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        match self {
            Ok(value) => Ok(value),
            Err(err) => Err(ActyxClientError::from((context().into(), err))),
        }
    }
}

impl From<(String, reqwest::Error)> for ActyxClientError {
    fn from(e: (String, reqwest::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 101,
            context: e.0,
        }
    }
}

impl From<(String, serde_json::Error)> for ActyxClientError {
    fn from(e: (String, serde_json::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 102,
            context: e.0,
        }
    }
}

impl From<(String, serde_cbor::Error)> for ActyxClientError {
    fn from(e: (String, serde_cbor::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 102,
            context: e.0,
        }
    }
}
