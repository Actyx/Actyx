use crate::util::run_task;
use axlib::node_connection::connect;
use futures::FutureExt;
use neon::{
    context::{Context, FunctionContext},
    result::JsResult,
    types::JsUndefined,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    addr: String,
    timeout: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Res {
    peer: String,
}

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Res>(
        cx,
        Box::new(|mut tx, Args { addr, timeout }| {
            async move {
                let auth = addr.parse()?;
                let timeout = Duration::from_secs(timeout.unwrap_or(10));
                let peer = tokio::time::timeout(timeout, connect(&mut tx, auth)).await??;
                Ok(Res { peer: peer.to_string() })
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
