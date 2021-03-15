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
use reqwest::{Client, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    service::{
        EventService, NodeIdResponse, PublishRequest, PublishResponse, QueryRequest, QueryResponse,
        SubscribeMonotonicRequest, SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    OffsetMap,
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
    pub error: String,
    pub error_code: u16,
    pub context: String,
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    url: Url, // cannot_be_a_base() ensured to be false
    token_query: String,
}

const API_URI: &str = "http://localhost:4454/api/";
const API_PATH: &str = "v2/events";

fn ensure_base(url: Url) -> anyhow::Result<Url> {
    anyhow::ensure!(!url.cannot_be_a_base(), "{} is not a valid base address", url);
    Ok(url)
}

impl HttpClient {
    /// This will configure a connection to the local Event Service, either an ActyxOS node in development
    /// mode or the production ActyxOS node where the app is deployed (in particular, it will
    /// inspect the `AX_API_URI` environment variable and fall back to
    /// `http://localhost:4454/api/`).
    pub async fn default() -> anyhow::Result<HttpClient> {
        let client = Client::new();
        let url = std::env::var("AX_API_URI").unwrap_or_else(|_| API_URI.to_string());
        let url = Url::parse(url.as_str())?;
        let mut url = ensure_base(url)?;
        url.path_segments_mut().unwrap().push(API_PATH);
        Self::new(client, url).await
    }

    pub async fn new(client: Client, url: Url) -> anyhow::Result<HttpClient> {
        let mut auth_url = ensure_base(url.clone())?;
        auth_url.path_segments_mut().unwrap().pop_if_empty().push("auth");
        let token = async move {
            // FIXME do request
            "xxx"
        }
        .await;
        let token_query = format!("token={}", token);

        Ok(Self {
            client,
            url,
            token_query,
        })
    }

    fn url(&self, path: &str) -> Url {
        let mut url = self.url.clone();
        url.path_segments_mut().unwrap().pop_if_empty().push(path); // unwrap() because of ensure_base()
        url.set_query(Some(self.token_query.as_str()));
        url
    }

    async fn do_request(
        &self,
        f: impl Fn(&Client) -> RequestBuilder,
    ) -> std::result::Result<Response, HttpClientError> {
        let response = f(&self.client)
            .send()
            .await
            .context(|| format!("sending {:?}", f(&self.client)))?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let error_code = response.status().as_u16();
            Err(HttpClientError {
                error: response
                    .text()
                    .await
                    .context(|| format!("getting body for {} reply to {:?}", error_code, f(&self.client)))?,
                error_code,
                context: format!("sending {:?}", f(&self.client)),
            })
        }
    }
}

#[async_trait]
impl EventService for HttpClient {
    async fn node_id(&self) -> Result<NodeIdResponse> {
        let response = self.do_request(|c| c.get(self.url("node_id"))).await?;
        let bytes = response
            .bytes()
            .await
            .context(|| format!("getting body for GET {}", self.url("node_id")))?;
        Ok(serde_json::from_slice(bytes.as_ref()).context(|| {
            format!(
                "deserializing node_id response from {:?} received from GET {}",
                bytes,
                self.url("node_id")
            )
        })?)
    }

    async fn offsets(&self) -> Result<OffsetMap> {
        let response = self.do_request(|c| c.get(self.url("offsets"))).await?;
        let bytes = response
            .bytes()
            .await
            .context(|| format!("getting body for GET {}", self.url("offsets")))?;
        Ok(serde_json::from_slice(bytes.as_ref()).context(|| {
            format!(
                "deserializing offsets response from {:?} received from GET {}",
                bytes,
                self.url("offsets")
            )
        })?)
    }

    async fn publish(&self, request: PublishRequest) -> Result<PublishResponse> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self.do_request(|c| c.post(self.url("publish")).json(&body)).await?;
        let bytes = response
            .bytes()
            .await
            .context(|| format!("getting body for GET {}", self.url("publish")))?;
        Ok(serde_json::from_slice(bytes.as_ref()).context(|| {
            format!(
                "deserializing publish response from {:?} received from GET {}",
                bytes,
                self.url("publish")
            )
        })?)
    }

    async fn query(&self, request: QueryRequest) -> Result<BoxStream<'static, QueryResponse>> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self.do_request(|c| c.post(self.url("query")).json(&body)).await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }

    async fn subscribe(&self, request: SubscribeRequest) -> Result<BoxStream<'static, SubscribeResponse>> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self.do_request(|c| c.post(self.url("subscribe")).json(&body)).await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }

    async fn subscribe_monotonic(
        &self,
        request: SubscribeMonotonicRequest,
    ) -> Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self
            .do_request(|c| c.post(self.url("subscribe_monotonic")).json(&body))
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
            error: format!("{:?}", e.1),
            error_code: 101,
            context: e.0,
        }
    }
}

impl From<(String, serde_json::Error)> for HttpClientError {
    fn from(e: (String, serde_json::Error)) -> Self {
        Self {
            error: format!("{:?}", e.1),
            error_code: 102,
            context: e.0,
        }
    }
}

impl From<(String, serde_cbor::Error)> for HttpClientError {
    fn from(e: (String, serde_cbor::Error)) -> Self {
        Self {
            error: format!("{:?}", e.1),
            error_code: 102,
            context: e.0,
        }
    }
}
