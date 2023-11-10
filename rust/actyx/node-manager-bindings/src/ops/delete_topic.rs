use crate::util::run_task;
use axlib::{
    node_connection::{request_single, Task},
    util::formats::{ActyxOSCode, AdminRequest, AdminResponse, TopicDeleteResponse},
};
use futures::FutureExt;
use neon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    peer: String,
    topic: String,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, TopicDeleteResponse>(
        cx,
        Box::new(|mut tx, Args { peer, topic: name }| {
            async move {
                let peer_id = peer.parse()?;
                let result = request_single(
                    &mut tx,
                    move |tx| Task::Admin(peer_id, AdminRequest::TopicDelete { name }, tx),
                    filter!(AdminRequest::TopicDelete => AdminResponse::TopicDeleteResponse),
                )
                .await;
                match result {
                    Ok(content) => Ok(content),
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
                }
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
