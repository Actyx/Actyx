use anyhow::Result;
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
};
use url::Url;

use crate::{
    service::{
        AuthenticationResponse, FilesGetResponse, OffsetsResponse, Order, PublishRequest, PublishResponse, QueryOpts,
        QueryRequest, QueryResponse, SessionId, StartFrom, SubscribeMonotonicOpts, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeOpts, SubscribeRequest, SubscribeResponse,
    },
    AppManifest, NodeId, OffsetMap,
};

#[cfg(feature = "with-tokio")]
use rand::Rng;
#[cfg(feature = "with-tokio")]
use std::time::Duration;

pub struct AxOpts {
    pub url: url::Url,
    pub manifest: AppManifest,
}

impl AxOpts {
    /// Create an [`AxOpts`] with a custom URL and the default application manifest.
    ///
    /// This function is similar to:
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
    /// This function is equivalent to:
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
    /// The default URL is `https://localhost:4454`,
    /// for the default manifest see [`AppManifest`].
    fn default() -> Self {
        Self {
            url: url::Url::from_str("https://localhost:4454").unwrap(),
            manifest: Default::default(),
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
pub struct AxError {
    pub error: serde_json::Value,
    pub error_code: u16,
    pub context: String,
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

impl Debug for Ax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ax")
            .field("base_url", &self.base_url.as_str())
            .field("app_manifest", &self.app_manifest)
            .finish()
    }
}

impl ActyxClient {
    /// Returns known offsets across local and replicated streams.
    ///
    /// If an authorization error (code 401) is returned, it will try to re-authenticate.
    /// If the service is unavailable (code 503), this method will retry to perform the
    /// request up to 10 times with exponentially increasing delay - currently,
    /// this behavior is only available if the `with-tokio` feature is enabled.
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

    /// Publishes a set of new events.
    ///
    /// If an authorization error (code 401) is returned, it will try to re-authenticate.
    /// If the service is unavailable (code 503), this method will retry to perform the
    /// request up to 10 times with exponentially increasing delay - currently,
    /// this behavior is only available if the `with-tokio` feature is enabled.
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

    /// Query events known at the time the request was received by the service.
    ///
    /// If `opts.is_none()` then this call is equivalent to the following:
    /// ```no_run
    /// query(
    ///     query, // Your query
    ///     Some(
    ///         QueryOpts {
    ///             lower_bound: None,
    ///             upper_bound: None,
    ///             order: Order::Asc,
    ///         }
    ///     )
    /// )
    /// ```
    /// In plain english:
    ///
    /// If an authorization error (code 401) is returned, it will try to re-authenticate.
    /// If the service is unavailable (code 503), this method will retry to perform the
    /// request up to 10 times with exponentially increasing delay - currently,
    /// this behavior is only available if the `with-tokio` feature is enabled.
    async fn query<Q: Into<String> + Send>(
        &self,
        query: Q,
        opts: Option<QueryOpts>,
    ) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let request = if let Some(opts) = opts {
            QueryRequest {
                query: query.into(),
                lower_bound: opts.lower_bound,
                upper_bound: opts.upper_bound,
                order: opts.order,
            }
        } else {
            QueryRequest {
                query: query.into(),
                lower_bound: None,
                upper_bound: None,
                order: Order::Asc,
            }
        };
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

    /// Suscribe to events that are currently known by the service followed by new "live" events.
    ///
    /// If `opts.is_none()` then this call is equivalent to the following:
    /// ```no_run
    /// subscribe(
    ///     query, // Your query
    ///     Some(
    ///         SubscribeOpts { lower_bound: None }
    ///     )
    /// )
    /// ```
    /// In plain english: subscribe using the provided query, reading _all_ events from the current session.
    ///
    /// If an authorization error (code 401) is returned, it will try to re-authenticate.
    /// If the service is unavailable (code 503), this method will retry to perform the
    /// request up to 10 times with exponentially increasing delay - currently,
    /// this behavior is only available if the `with-tokio` feature is enabled.
    async fn subscribe<Q: Into<String> + Send>(
        &self,
        query: Q,
        opts: Option<SubscribeOpts>,
    ) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let request = SubscribeRequest {
            query: query.into(),
            lower_bound: opts.map(|opts| opts.lower_bound).unwrap_or_default(),
        };
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

    /// Subscribe to events that are currently known by the service followed by new "live" events until
    /// the service learns about events that need to be sorted earlier than an event already received.
    ///
    /// If `opts.is_none()` then this call is equivalent to the following:
    /// ```no_run
    /// subscribe_monotonic(
    ///     query, // Your query
    ///     Some(
    ///         SubscribeMonotonicOpts {
    ///             from: StartFrom::LowerBound(OffsetMap::empty()),
    ///             session: SessionId::from("me")
    ///         }
    ///     )
    /// )
    /// ```
    /// In plain english: subscribe using the provided query, reading _all_ events from the current session.
    ///
    /// If an authorization error (code 401) is returned, it will try to re-authenticate.
    /// If the service is unavailable (code 503), this method will retry to perform the
    /// request up to 10 times with exponentially increasing delay - currently,
    /// this behavior is only available if the `with-tokio` feature is enabled.
    async fn subscribe_monotonic<Q: Into<String> + Send>(
        &self,
        query: Q,
        opts: Option<SubscribeMonotonicOpts>,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let request = if let Some(opts) = opts {
            SubscribeMonotonicRequest {
                query: query.into(),
                from: opts.from,
                session: opts.session,
            }
        } else {
            SubscribeMonotonicRequest {
                query: query.into(),
                from: StartFrom::LowerBound(OffsetMap::empty()),
                session: SessionId::from("me"),
            }
        };
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
    AxError: From<(String, E)>,
{
    type Output = std::result::Result<T, AxError>;

    #[inline]
    fn context<F, C>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        match self {
            Ok(value) => Ok(value),
            Err(err) => Err(AxError::from((context().into(), err))),
        }
    }
}

impl From<(String, reqwest::Error)> for AxError {
    fn from(e: (String, reqwest::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 101,
            context: e.0,
        }
    }
}

impl From<(String, serde_json::Error)> for AxError {
    fn from(e: (String, serde_json::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 102,
            context: e.0,
        }
    }
}

impl From<(String, serde_cbor::Error)> for AxError {
    fn from(e: (String, serde_cbor::Error)) -> Self {
        Self {
            error: serde_json::json!(format!("{:?}", e.1)),
            error_code: 102,
            context: e.0,
        }
    }
}
