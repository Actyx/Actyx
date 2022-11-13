use crate::util::run_task;
use actyx_sdk::{
    service::{Diagnostic, EventResponse, Order, QueryRequest},
    Payload,
};
use axlib::node_connection::{request_events, Task};
use futures::{channel::mpsc::Sender, FutureExt, StreamExt};
use libp2p::PeerId;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use util::formats::{ax_err, events_protocol::EventsRequest, ActyxOSCode, ActyxOSResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventDiagnostic {
    Event(EventResponse<Payload>),
    Diagnostic(Diagnostic),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    peer: String,
    query: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Res {
    events: Option<Vec<EventDiagnostic>>,
}

async fn do_query(mut tx: Sender<Task>, peer: PeerId, query: String) -> ActyxOSResult<Res> {
    let r = request_events(
        &mut tx,
        peer,
        EventsRequest::Query(QueryRequest {
            lower_bound: None,
            upper_bound: None,
            query,
            order: Order::Asc,
        }),
    )
    .await;

    match r {
        Err(err) if err.code() == ActyxOSCode::ERR_UNSUPPORTED => Ok(Res { events: None }),
        Err(err) => ax_err(
            util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
            format!("EventsRequests::Query returned unexpected error: {:?}", err),
        ),
        Ok(mut stream) => {
            async {
                let mut events = Vec::new();
                while let Some(ev) = stream.next().await {
                    events.push(EventDiagnostic::Event(ev?));
                }
                Ok(Res { events: Some(events) })
            }
            .await
        }
    }
}

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Res>(
        cx,
        Box::new(|tx, Args { peer, query }| {
            async move {
                let peer_id = peer.parse::<PeerId>()?;
                let res = do_query(tx, peer_id, query).await;
                match res {
                    Err(e) if e.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => {
                        eprintln!("unable to reach node {}", peer);
                        Err(anyhow::anyhow!(e))
                    }
                    Err(e) if e.code() == ActyxOSCode::ERR_UNAUTHORIZED => {
                        eprintln!("not authorized with node {}", peer);
                        Err(anyhow::anyhow!(e))
                    }
                    Err(e) => {
                        eprintln!("error querying node {}: {}", peer, e);
                        Err(anyhow::anyhow!(e))
                    }
                    Ok(res) => Ok(res),
                }
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
