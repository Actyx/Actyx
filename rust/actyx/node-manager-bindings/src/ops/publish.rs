use crate::util::run_task;
use actyx_sdk::service::{PublishEvent, PublishRequest, PublishResponse};
use axlib::node_connection::{publish as publish_impl, Task};
use futures::{channel::mpsc::Sender, FutureExt, StreamExt};
use libp2p::PeerId;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use util::formats::{ax_err, events_protocol::EventsRequest, ActyxOSCode, ActyxOSResult};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    peer: String,
    events: Vec<PublishEvent>,
}

async fn publish(mut tx: Sender<Task>, peer: PeerId, data: Vec<PublishEvent>) -> ActyxOSResult<PublishResponse> {
    let r = publish_impl(&mut tx, peer, EventsRequest::Publish(PublishRequest { data })).await;

    match r {
        Err(err) => ax_err(
            util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
            format!("EventsRequests::Publish returned unexpected error: {:?}", err),
        ),
        Ok(mut stream) => {
            async {
                let Some(result) = stream.next().await else {
                    return ax_err(
                        util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
                        "EventsRequests::Publish returned empty".to_string(),
                    );
                };

                let response = match result {
                    Err(err) => return Err(err),
                    Ok(x) => x,
                };

                Ok(response)
            }
            .await
        }
    }
}

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, PublishResponse>(
        cx,
        Box::new(|tx, Args { events, peer }| {
            async move {
                let peer_id = peer.parse::<PeerId>()?;
                let res = publish(tx, peer_id, events).await;
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
                        eprintln!("error publishing node {}: {}", peer, e);
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
