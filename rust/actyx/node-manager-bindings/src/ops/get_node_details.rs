use futures::FutureExt;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;
use util::formats::events_protocol::{EventsRequest, EventsResponse};
use util::formats::{ActyxOSCode, ActyxOSResult, AdminRequest, AdminResponse};

use crate::consts::DEFAULT_TIMEOUT_SEC;
use crate::types::*;
use crate::util::*;
use axlib::node_connection::{request_single, Task};
use futures::channel::mpsc::Sender;
use libp2p::PeerId;

async fn get_node_details(
    mut tx: Sender<Task>,
    peer: PeerId,
    timeout: Duration,
) -> ActyxOSResult<ConnectedNodeDetails> {
    println!("getting details for node {:?} (timeout: {})", peer, timeout.as_secs());

    let status = request_single(
        &mut tx,
        move |tx| Task::Admin(peer, AdminRequest::NodesLs, tx),
        filter!(AdminRequest::NodesLs => AdminResponse::NodesLsResponse),
    )
    .await?;

    let settings = request_single(
        &mut tx,
        move |tx| {
            Task::Admin(
                peer,
                AdminRequest::SettingsGet {
                    scope: settings::Scope {
                        tokens: vec!["com.actyx".to_string()],
                    },
                    no_defaults: false,
                },
                tx,
            )
        },
        filter!(AdminRequest::SettingsGet => AdminResponse::SettingsGetResponse),
    )
    .await?;

    let settings_schema = request_single(
        &mut tx,
        move |tx| {
            Task::Admin(
                peer,
                AdminRequest::SettingsSchema {
                    scope: settings::Scope {
                        tokens: vec!["com.actyx".to_string()],
                    },
                },
                tx,
            )
        },
        filter!(AdminRequest::SettingsSchema => AdminResponse::SettingsSchemaResponse),
    )
    .await?;

    let offsets = request_single(
        &mut tx,
        move |tx| Task::Events(peer, EventsRequest::Offsets, tx),
        filter!(EventsRequest::Offsets => EventsResponse::Offsets),
    )
    .await?;

    // The NodesInspect can easily time out if the store is starting up. Instead of
    // returning an unreachable node in this case, we return the swarm state as none.
    let swarm = request_single(
        &mut tx,
        move |tx| Task::Admin(peer, AdminRequest::NodesInspect, tx),
        filter!(AdminRequest::NodesInspect => AdminResponse::NodesInspectResponse),
    )
    .await
    .ok();

    let addrs = swarm.as_ref().map(|s| s.admin_addrs.join(", "));
    Ok(ConnectedNodeDetails {
        node_id: status.node_id,
        display_name: status.display_name,
        started_iso: status.started_iso,
        started_unix: status.started_unix,
        version: format!("{}", status.version),
        addrs,
        swarm_state: swarm,
        settings_schema,
        settings,
        offsets: Some(offsets),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    peer: String,
    timeout: Option<u64>,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Node>(
        cx,
        Box::new(|tx, Args { peer, timeout }| {
            async move {
                let res = get_node_details(
                    tx,
                    peer.parse().unwrap(),
                    Duration::from_secs(timeout.unwrap_or(DEFAULT_TIMEOUT_SEC)),
                )
                .await;

                match res {
                    Err(e) if e.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => Ok(Node::DisconnectedNode { peer }),
                    Err(e) if e.code() == ActyxOSCode::ERR_UNAUTHORIZED => Ok(Node::UnauthorizedNode { peer }),
                    Ok(details) => Ok(Node::ReachableNode { peer, details }),
                    Err(e) => {
                        eprintln!("error getting node details: {}", e);
                        Err(anyhow::anyhow!(e))
                    }
                }
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
