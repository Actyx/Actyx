use crate::{types::Nothing, util::run_task};
use ax_core::{
    node_connection::{request, Task},
    util::formats::AdminRequest,
};
use futures::FutureExt;
use neon::{
    context::{Context, FunctionContext},
    result::JsResult,
    types::JsUndefined,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    peer: String,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Nothing>(
        cx,
        Box::new(|mut tx, Args { peer }| {
            async move {
                println!("shutting down node {}", peer);
                let peer_id = peer.parse()?;
                request(
                    &mut tx,
                    move |tx| Task::Admin(peer_id, AdminRequest::NodesShutdown, tx),
                    Ok,
                )
                .await?;
                Ok(Nothing {})
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
