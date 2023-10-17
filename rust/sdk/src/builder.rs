use std::future;

use futures::{stream::BoxStream, StreamExt};

use crate::{
    app_id,
    client::{to_lines, WithContext},
    service::{
        Order, QueryRequest, QueryResponse, SessionId, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    ActyxClient, OffsetMap,
};

struct Builder(ActyxClient);

impl Builder {
    pub fn query<Q: Into<String> + Send>(self, query: Q) -> Query {
        Query::new(self.0, query)
    }

    pub fn subscribe<Q: Into<String> + Send>(self, query: Q) -> Subscribe {
        Subscribe::new(self.0, query)
    }

    pub fn subscribe_monotonic<Q: Into<String> + Send>(self, query: Q) -> SubscribeMonotonic {
        SubscribeMonotonic::new(self.0, query)
    }
}

struct Query {
    client: ActyxClient,
    request: QueryRequest,
}

impl Query {
    fn new<Q: Into<String>>(client: ActyxClient, query: Q) -> Self {
        Self {
            client,
            request: QueryRequest {
                query: query.into(),
                lower_bound: Some(OffsetMap::empty()),
                upper_bound: None,
                order: Order::Asc,
            },
        }
    }

    pub fn with_lower_bound(mut self, lower_bound: OffsetMap) -> Self {
        self.request.lower_bound = Some(lower_bound);
        self
    }

    pub fn with_upper_bound(mut self, upper_bound: OffsetMap) -> Self {
        self.request.upper_bound = Some(upper_bound);
        self
    }

    pub fn with_order(mut self, order: Order) -> Self {
        self.request.order = order;
        self
    }

    pub async fn execute(self) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let body = serde_json::to_value(&self.request).context(|| format!("serializing {:?}", &self.request))?;
        let response = self
            .client
            .do_request(|c| c.post(self.client.events_url("query")).json(&body))
            .await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }
}

struct Subscribe {
    client: ActyxClient,
    request: SubscribeRequest,
}

impl Subscribe {
    fn new<Q: Into<String>>(client: ActyxClient, query: Q) -> Self {
        Self {
            client,
            request: SubscribeRequest {
                query: query.into(),
                lower_bound: Some(OffsetMap::empty()),
            },
        }
    }

    fn with_lower_bound(mut self, lower_bound: OffsetMap) -> Self {
        self.request.lower_bound = Some(lower_bound);
        self
    }

    async fn execute(self) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let body = serde_json::to_value(&self.request).context(|| format!("serializing {:?}", &self.request))?;
        let response = self
            .client
            .do_request(|c| c.post(self.client.events_url("subscribe")).json(&body))
            .await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }
}

struct SubscribeMonotonic {
    client: ActyxClient,
    request: SubscribeMonotonicRequest,
}

impl SubscribeMonotonic {
    fn new<Q: Into<String>>(client: ActyxClient, query: Q) -> Self {
        Self {
            client,
            request: SubscribeMonotonicRequest {
                query: query.into(),
                session: SessionId::from("me"),
                from: StartFrom::LowerBound(OffsetMap::empty()),
            },
        }
    }

    fn with_session_id<T: Into<SessionId>>(mut self, session_id: T) -> Self {
        self.request.session = session_id.into();
        self
    }

    fn with_start_from(mut self, start_from: StartFrom) -> Self {
        self.request.from = start_from;
        self
    }

    async fn execute(self) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let body = serde_json::to_value(&self.request).context(|| format!("serializing {:?}", &self.request))?;
        let response = self
            .client
            .do_request(|c| c.post(self.client.events_url("subscribe_monotonic")).json(&body))
            .await?;
        let res = to_lines(response.bytes_stream())
            .map(|bs| serde_json::from_slice(bs.as_ref()))
            // FIXME this swallows deserialization errors, silently dropping event envelopes
            .filter_map(|res| future::ready(res.ok()));
        Ok(res.boxed())
    }
}

async fn test() {
    let client = ActyxClient::new(
        "localhost:4454".parse().unwrap(),
        crate::AppManifest {
            app_id: app_id!("com.example.hey"),
            display_name: "Test".to_string(),
            version: "0.0.1".to_string(),
            signature: None,
        },
    )
    .await
    .unwrap();
    let builder = Builder(client);
    builder
        .query("FROM allEvents")
        .with_order(Order::Desc)
        .execute()
        .await
        .unwrap();
}
