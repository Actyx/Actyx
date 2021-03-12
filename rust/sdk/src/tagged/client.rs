use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use derive_more::{Display, Error};
use futures::{future, stream::iter, Stream, StreamExt};
use reqwest::{Client, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::service;

/// Error type that is returned in the response body by the Event Service when requests fail
///
/// The Event Service does not map client errors or internal errors to HTTP status codes,
/// instead it gives more structured information using this data type, except when the request
/// is not understood at all.
#[derive(Clone, Debug, Error, Display, Serialize, Deserialize, PartialEq)]
#[display(fmt = "error {} while {}: {}", error_code, context, error)]
#[serde(rename_all = "camelCase")]
pub struct EventServiceError {
    pub error: String,
    pub error_code: u16,
    pub context: String,
}

#[derive(Clone)]
pub struct EventService {
    client: Client,
    url: Url,
}

impl EventService {
    fn url(&self, path: &str) -> Url {
        self.url.join(path).unwrap()
    }

    async fn do_request(
        &self,
        f: impl Fn(&Client) -> RequestBuilder,
    ) -> std::result::Result<Response, EventServiceError> {
        let response = f(&self.client)
            .send()
            .await
            .context(|| format!("sending {:?}", f(&self.client)))?;
        if response.status().is_success() {
            Ok(response)
        } else {
            let error_code = response.status().as_u16();
            Err(EventServiceError {
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

impl Default for EventService {
    /// This will configure a connection to the local Event Service, either an ActyxOS node in development
    /// mode or the production ActyxOS node where the app is deployed (in particular, it will
    /// inspect the `AX_API_URI` environment variable and fall back to
    /// `http://localhost:4454/api/`).
    fn default() -> Self {
        let client = Client::new();
        let url = std::env::var("AX_API_URI")
            .and_then(|mut uri| {
                if !uri.ends_with('/') {
                    uri.push('/')
                };
                Url::parse(&*uri).map_err(|_| std::env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| Url::parse("http://localhost:4454/api/").unwrap())
            .join("v2/events/")
            .unwrap();
        EventService { client, url }
    }
}

#[async_trait]
impl service::EventService for EventService {
    async fn node_id(&self) -> Result<service::NodeIdResponse> {
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

    async fn offsets(&self) -> Result<crate::OffsetMap> {
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

    async fn publish(&self, request: service::PublishRequest) -> Result<service::PublishResponse> {
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

    async fn query(
        &self,
        request: service::QueryRequest,
    ) -> Result<futures::stream::BoxStream<'static, service::QueryResponse>> {
        let body = serde_json::to_value(&request).context(|| format!("serializing {:?}", &request))?;
        let response = self.do_request(|c| c.post(self.url("query")).json(&body)).await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }

    async fn subscribe(
        &self,
        request: service::SubscribeRequest,
    ) -> Result<futures::stream::BoxStream<'static, service::SubscribeResponse>> {
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
        request: service::SubscribeMonotonicRequest,
    ) -> Result<futures::stream::BoxStream<'static, service::SubscribeMonotonicResponse>> {
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
    EventServiceError: From<(String, E)>,
{
    type Output = std::result::Result<T, EventServiceError>;

    #[inline]
    fn context<F, C>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        match self {
            Ok(value) => Ok(value),
            Err(err) => Err(EventServiceError::from((context().into(), err))),
        }
    }
}

impl From<(String, reqwest::Error)> for EventServiceError {
    fn from(e: (String, reqwest::Error)) -> Self {
        Self {
            error: format!("{:?}", e.1),
            error_code: 101,
            context: e.0,
        }
    }
}

impl From<(String, serde_json::Error)> for EventServiceError {
    fn from(e: (String, serde_json::Error)) -> Self {
        Self {
            error: format!("{:?}", e.1),
            error_code: 102,
            context: e.0,
        }
    }
}

impl From<(String, serde_cbor::Error)> for EventServiceError {
    fn from(e: (String, serde_cbor::Error)) -> Self {
        Self {
            error: format!("{:?}", e.1),
            error_code: 102,
            context: e.0,
        }
    }
}
