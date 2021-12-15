use axlib::{node_connection::NodeConnection, private_key::AxPrivateKey};
use futures::StreamExt;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{convert::TryInto, str::FromStr};
use tokio::time::Duration;
use util::formats::events_protocol::{EventsRequest, EventsResponse};
use util::formats::{ax_err, ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse};

use crate::consts::DEFAULT_TIMEOUT_SEC;
use crate::types::*;
use crate::util::*;

macro_rules! filter {
    ($req:path => $res:path) => {
        |res| match res {
            $res(r) => Ok(r),
            r => ax_err(
                util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
                format!("{} returned mismatched response: {:?}", stringify!($req), r),
            ),
        }
    };
}

async fn get_node_details(
    key: &AxPrivateKey,
    node: NodeConnection,
    timeout: Duration,
) -> ActyxOSResult<ConnectedNodeDetails> {
    println!("getting details for node {:?} (timeout: {})", node, timeout.as_secs());
    // We first establis the connection with a configurable timeout. Note that further
    // requests, except for inspect, fail according to the connection.
    let mut conn = tokio::time::timeout(timeout, node.connect(key))
        .await
        .map_err(|_elapsed| {
            println!("connecting to node timed out");
            ActyxOSError::new(
                ActyxOSCode::ERR_NODE_UNREACHABLE,
                format!("node {:?} didn't respond within {} seconds", node, timeout.as_secs()),
            )
        })
        .and_then(|x| x)?;

    //println!("{}: node status: getting...", request_id);
    let status = conn
        .request(AdminRequest::NodesLs)
        .await
        .and_then(filter!(AdminRequest::NodesLs => AdminResponse::NodesLsResponse))?;

    let settings = conn
        .request(AdminRequest::SettingsGet {
            scope: settings::Scope {
                tokens: vec!["com.actyx".to_string()],
            },
            no_defaults: false,
        })
        .await
        .and_then(filter!(AdminRequest::SettingsGet => AdminResponse::SettingsGetResponse))?;

    let settings_schema = conn
        .request(AdminRequest::SettingsSchema {
            scope: settings::Scope {
                tokens: vec!["com.actyx".to_string()],
            },
        })
        .await
        .and_then(filter!(AdminRequest::SettingsSchema => AdminResponse::SettingsSchemaResponse))?;

    let offsets = conn
        .request_events(EventsRequest::Offsets)
        .await?
        .next()
        .await
        .ok_or_else(|| {
            ActyxOSError::new(
                ActyxOSCode::ERR_INTERNAL_ERROR,
                "unexpected stream end when querying offsets",
            )
        })
        .and_then(filter!(EventsRequest::Offsets => EventsResponse::Offsets))?;

    // The NodesInspect can easily time out if the store is starting up. Instead of
    // returning an unreachable node in this case, we return the swarm state as none.
    let swarm = conn
        .request(AdminRequest::NodesInspect)
        .await
        .and_then(filter!(AdminRequest::NodesInspect => AdminResponse::NodesInspectResponse))
        .ok();

    let addrs = swarm.clone().map(|s| s.admin_addrs.join(", "));
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
    addr: String,
    timeout: Option<u64>,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Node>(
        cx,
        Arc::new(|Args { addr, timeout }| {
            let private_key: AxPrivateKey = (&None).try_into()?;
            let node_connection = NodeConnection::from_str(addr.as_str())?;
            let res = run_ft(get_node_details(
                &private_key,
                node_connection,
                Duration::from_secs(timeout.unwrap_or(DEFAULT_TIMEOUT_SEC)),
            ));

            match res {
                Err(e) if e.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => {
                    eprintln!("returning unreachable node {}", addr);
                    Ok(Node::UnreachableNode { addr })
                }
                Err(e) if e.code() == ActyxOSCode::ERR_UNAUTHORIZED => Ok(Node::UnauthorizedNode { addr }),
                Ok(details) => Ok(Node::ReachableNode { addr, details }),
                Err(e) => {
                    eprintln!("error getting node details: {}", e);
                    Err(anyhow::anyhow!(e))
                }
            }
        }),
    );
    Ok(ud)
}
