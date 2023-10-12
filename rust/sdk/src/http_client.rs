use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use derive_more::{Display, Error};
use futures::{
    future,
    stream::{iter, BoxStream, Stream, StreamExt},
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
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};
use url::Url;

use crate::{
    service::{
        AuthenticationResponse, EventService, FilesGetResponse, OffsetsResponse, PublishRequest, PublishResponse,
        QueryRequest, QueryResponse, SubscribeMonotonicRequest, SubscribeMonotonicResponse, SubscribeRequest,
        SubscribeResponse,
    },
    AppManifest, NodeId,
};
use rand::Rng;

/// Error type that is returned in the response body by the Event Service when requests fail
///
/// The Event Service does not map client errors or internal errors to HTTP status codes,
/// instead it gives more structured information using this data type, except when the request
/// is not understood at all.
#[derive(Clone, Debug, Error, Display, Serialize, Deserialize, PartialEq, Eq)]
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
    node_id: NodeId,
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

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn events_url(&self, path: &str) -> Url {
        // Safe to unwrap, because we fully control path creation
        self.base_url.join(&format!("events/{}", path)).unwrap()
    }

    fn files_url(&self) -> Url {
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
    async fn do_request(&self, f: impl FnOnce(&Client) -> RequestBuilder) -> anyhow::Result<Response> {
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
            Err(HttpClientError {
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
        let cid = Cid::from_str(&hash).map_err(|e| HttpClientError {
            error: serde_json::Value::String(e.to_string()),
            error_code: 102,
            context: format!("Tried to parse {} into a Cid", hash),
        })?;
        Ok(cid)
    }

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
