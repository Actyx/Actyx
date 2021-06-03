/*
 * Copyright 2021 Actyx AG
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
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use derive_more::{Display, Error};
use futures::{
    future,
    stream::{iter, BoxStream, Stream, StreamExt},
};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};
use url::Url;

use crate::{
    service::{
        AuthenticationResponse, EventService, OffsetsResponse, PublishRequest, PublishResponse, QueryRequest,
        QueryResponse, SubscribeMonotonicRequest, SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    AppManifest, NodeId,
};

/// Error type that is returned in the response body by the Event Service when requests fail
///
/// The Event Service does not map client errors or internal errors to HTTP status codes,
/// instead it gives more structured information using this data type, except when the request
/// is not understood at all.
#[derive(Clone, Debug, Error, Display, Serialize, Deserialize, PartialEq)]
#[display(fmt = "error {} while {}: {}", error_code, context, error)]
#[serde(rename_all = "camelCase")]
pub struct HttpClientError {
    pub error: serde_json::Value,
    pub error_code: u16,
    pub context: String,
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    base_url: Url,
    token: Arc<RwLock<String>>,
    app_manifest: AppManifest,
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

impl HttpClient {
    /// Configures connection to Actyx node with provided Url and AppManifest.
    /// All path segments of the Url (if any) are discarded.
    pub async fn new(origin: Url, app_manifest: AppManifest) -> anyhow::Result<Self> {
        anyhow::ensure!(!origin.cannot_be_a_base(), "{} is not a valid base address", origin);
        let mut base_url = origin;
        base_url.set_path("api/v2/");
        let client = Client::new();

        let token = get_token(&client, &base_url, &app_manifest).await?;

        Ok(Self {
            client,
            base_url,
            token: Arc::new(RwLock::new(token)),
            app_manifest,
        })
    }

    pub async fn node_id(&self) -> anyhow::Result<NodeId> {
        let txt = self
            .client
            .get(self.base_url.join("node/id").unwrap())
            .send()
            .await?
            .text()
            .await
            .context(|| "getting body for GET node/id")?;
        let res: NodeId = txt.parse()?;
        Ok(res)
    }

    fn events_url(&self, path: &str) -> Url {
        // Safe to unwrap, because we fully control path creation
        self.base_url.join(&format!("events/{}", path)).unwrap()
    }

    async fn re_authenticate(&self) -> anyhow::Result<String> {
        let token = get_token(&self.client, &self.base_url, &self.app_manifest).await?;
        let mut write_guard = self.token.write().unwrap();
        *write_guard = token.clone();
        Ok(token)
    }

    async fn send_request(
        &self,
        f: impl Fn(&Client) -> RequestBuilder,
        token: &str,
    ) -> std::result::Result<Response, HttpClientError> {
        f(&self.client)
            .header("Authorization", &format!("Bearer {}", token))
            .send()
            .await
            .context(|| format!("sending {:?}", f(&self.client)))
    }

    /// Makes request to Actyx apis. On http authorization error tries to
    /// re-authenticate and retries the request once.
    async fn do_request(&self, f: impl Fn(&Client) -> RequestBuilder) -> anyhow::Result<Response> {
        let token = {
            let read_guard = self.token.read().unwrap();
            let token = &*read_guard;
            token.clone()
        };

        let mut response = self.send_request(&f, &token).await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            let token = self.re_authenticate().await?;
            response = self.send_request(&f, &token).await?;
        }

        if response.status().is_success() {
            Ok(response)
        } else {
            let error_code = response.status().as_u16();
            Err(HttpClientError {
                error: response
                    .json()
                    .await
                    .context(|| format!("getting body for {} reply to {:?}", error_code, f(&self.client)))?,
                error_code,
                context: format!("sending {:?}", f(&self.client)),
            }
            .into())
        }
    }
}

impl Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("base_url", &self.base_url.as_str())
            .field("app_manifest", &self.app_manifest)
            .finish()
    }
}

#[async_trait]
impl EventService for HttpClient {
    async fn offsets(&self) -> anyhow::Result<OffsetsResponse> {
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

    async fn publish(&self, request: PublishRequest) -> anyhow::Result<PublishResponse> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self
            .do_request(|c| c.post(self.events_url("publish")).json(&body))
            .await?;
        let bytes = response
            .bytes()
            .await
            .context(|| format!("getting body for GET {}", self.events_url("publish")))?;
        Ok(serde_json::from_slice(bytes.as_ref()).context(|| {
            format!(
                "deserializing publish response from {:?} received from GET {}",
                bytes,
                self.events_url("publish")
            )
        })?)
    }

    async fn query(&self, request: QueryRequest) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self
            .do_request(|c| c.post(self.events_url("query")).json(&body))
            .await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }

    async fn subscribe(&self, request: SubscribeRequest) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self
            .do_request(|c| c.post(self.events_url("subscribe")).json(&body))
            .await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }

    async fn subscribe_monotonic(
        &self,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self
            .do_request(|c| c.post(self.events_url("subscribe_monotonic")).json(&body))
            .await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
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

pub(crate) trait WithContext {
    type Output;
    fn context<F, T>(self, context: F) -> Self::Output
    where
        T: Into<String>,
        F: FnOnce() -> T;
}
impl<T, E> WithContext for std::result::Result<T, E>
where
    HttpClientError: From<(String, E)>,
{
    type Output = std::result::Result<T, HttpClientError>;

    #[inline]
    fn context<F, C>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        match self {
            Ok(value) => Ok(value),
            Err(err) => Err(HttpClientError::from((context().into(), err))),
        }
    }
}

impl From<(String, reqwest::Error)> for HttpClientError {
    fn from(e: (String, reqwest::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 101,
            context: e.0,
        }
    }
}

impl From<(String, serde_json::Error)> for HttpClientError {
    fn from(e: (String, serde_json::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 102,
            context: e.0,
        }
    }
}

impl From<(String, serde_cbor::Error)> for HttpClientError {
    fn from(e: (String, serde_cbor::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 102,
            context: e.0,
        }
    }
}
