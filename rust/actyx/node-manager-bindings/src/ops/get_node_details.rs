use axlib::{cmd::KeyPathWrapper, node_connection::NodeConnection, private_key::AxPrivateKey};
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{convert::TryInto, str::FromStr};
use tokio::time::Duration;
use util::formats::{ax_err, ActyxOSCode, ActyxOSResult, AdminRequest, AdminResponse};

use crate::types::*;
use crate::util::*;

async fn get_node_details(
    key: &AxPrivateKey,
    node: NodeConnection,
    timeout: Duration,
) -> ActyxOSResult<ConnectedNodeDetails> {
    println!("getting details for node {:?}", node);
    let mut status_conn = node.clone();
    let status_f = status_conn.request(key, AdminRequest::NodesLs);
    let mut settings_conn = node.clone();
    let settings_f = settings_conn.request(
        key,
        AdminRequest::SettingsGet {
            scope: settings::Scope {
                tokens: vec!["com.actyx".to_string()],
            },
            no_defaults: false,
        },
    );
    let mut settings_schema_conn = node.clone();
    let settings_schema_f = settings_schema_conn.request(
        key,
        AdminRequest::SettingsSchema {
            scope: settings::Scope {
                tokens: vec!["com.actyx".to_string()],
            },
        },
    );
    let mut swarm_conn = node.clone();
    let swarm_f = swarm_conn.request(key, AdminRequest::NodesInspect);

    // Parallel connections to the same node don't work!
    //let (status_res, settings_res, settings_schema_res, swarm_res): (
    //    RequestResult,
    //    RequestResult,
    //    RequestResult,
    //    RequestResult,
    //) = join!(status_f, settings_f, settings_schema_f, swarm_f);

    // We use the first request to timeout and exit with ERR_NODE_UNREACHABLE. The other
    // requests will also time out, but possibly after a long time. This should only ocurr
    // if the node stops responding between these requests.

    //let status_res = tokio::time::timeout(timeout, status_f).await;
    let status_res = tokio::time::timeout(timeout, status_f).await;
    let status = match status_res {
        Ok(v) => v?,
        Err(_) => {
            return ax_err(
                util::formats::ActyxOSCode::ERR_NODE_UNREACHABLE,
                format!("node didn't respond within {} seconds", timeout.as_secs()),
            )
        }
    };
    let status = match status {
        AdminResponse::NodesLsResponse(r) => Ok(r),
        r => ax_err(
            util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
            format!("AdminRequest::NodeLs returned mismatched response: {:?}", r),
        ),
    }?;

    let settings = settings_f.await?;
    let settings = match settings {
        AdminResponse::SettingsGetResponse(r) => Ok(r),
        r => ax_err(
            util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
            format!("AdminRequest::SettingsGet returned mismatched response: {:?}", r),
        ),
    }?;

    let settings_schema = settings_schema_f.await?;
    let settings_schema = match settings_schema {
        AdminResponse::SettingsSchemaResponse(r) => Ok(r),
        r => ax_err(
            util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
            format!(
                "AdminRequest::SettingsSchemaResponse returned mismatched response: {:?}",
                r
            ),
        ),
    }?;

    let swarm = swarm_f.await?;
    let swarm = match swarm {
        AdminResponse::NodesInspectResponse(r) => Ok(r),
        r => ax_err(
            util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
            format!("InternalRequest::GetSwarmState returned mismatched response: {:?}", r),
        ),
    }?;

    let addrs = swarm.admin_addrs.join(", ");
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
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    addr: String,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Node>(
        cx,
        Arc::new(|Args { addr }| {
            let private_key: AxPrivateKey = None::<KeyPathWrapper>.try_into()?;
            let node_connection = NodeConnection::from_str(addr.as_str())?;
            let res = run_ft(get_node_details(&private_key, node_connection, Duration::from_secs(2)))?;

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
