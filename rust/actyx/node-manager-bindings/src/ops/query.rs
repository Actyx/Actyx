use crate::util::{default_private_key, node_connection, run_ft, run_task};
use actyx_sdk::{
    language::Query,
    service::{Diagnostic, EventResponse, Order, QueryRequest},
    Payload,
};
use anyhow::anyhow;
use axlib::{node_connection::NodeConnection, private_key::AxPrivateKey};
use futures::{future::ready, StreamExt};
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use util::formats::{
    ax_err,
    events_protocol::{EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSResult,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventDiagnostic {
    Event(EventResponse<Payload>),
    Diagnostic(Diagnostic),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    addr: String,
    query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Res {
    events: Option<Vec<EventDiagnostic>>,
}

async fn do_query(key: &AxPrivateKey, mut node: NodeConnection, query: Query) -> ActyxOSResult<Res> {
    let r = node.request_events(
        &key,
        EventsRequest::Query(QueryRequest {
            lower_bound: None,
            upper_bound: None,
            query,
            order: Order::Asc,
        }),
    );

    match r.await {
        Err(err) if err.code() == ActyxOSCode::ERR_UNSUPPORTED => ready(Ok(Res { events: None })).await,
        Err(err) => {
            ready(ax_err(
                util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
                format!("EventsRequests::Query returned unexpected error: {:?}", err),
            ))
            .await
        }
        Ok(stream) => {
            async {
                let events: Vec<EventsResponse> = stream.collect().await;
                let appropriate: Vec<EventDiagnostic> = events
                    .iter()
                    .filter_map(|r| match r {
                        EventsResponse::Diagnostic(d) => Some(EventDiagnostic::Diagnostic(d.clone())),
                        EventsResponse::Event(e) => Some(EventDiagnostic::Event(e.clone())),
                        e => {
                            eprintln!("got unexpected response {:?}", e);
                            None
                        }
                    })
                    .collect();
                Ok(Res {
                    events: Some(appropriate),
                })
            }
            .await
        }
    }
}

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Res>(
        cx,
        Arc::new(|Args { addr, query }| {
            let parsed_query = match query.parse::<Query>() {
                Ok(q) => Ok(q),
                Err(err) => ax_err(
                    ActyxOSCode::ERR_INVALID_INPUT,
                    format!("unable to parse query: {}", err),
                ),
            }?;

            let key = default_private_key().map_err(|e| anyhow!("error getting default key: {}", e))?;
            let node = node_connection(&addr).map_err(|e| anyhow!("error connecting to node {}: {}", addr, e))?;
            let res = run_ft(do_query(&key, node, parsed_query))?;
            match res {
                Err(e) if e.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => {
                    eprintln!("unable to reach node {}", addr);
                    Err(anyhow::anyhow!(e))
                }
                Err(e) if e.code() == ActyxOSCode::ERR_UNAUTHORIZED => {
                    eprintln!("not authorized with node {}", addr);
                    Err(anyhow::anyhow!(e))
                }
                Err(e) => {
                    eprintln!("error querying node {}: {}", addr, e);
                    Err(anyhow::anyhow!(e))
                }
                Ok(res) => Ok(res),
            }
        }),
    );
    Ok(ud)
}
