use crate::{
    types::Nothing,
    util::{default_private_key, node_connection, run_ft, run_task},
};
use anyhow::anyhow;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    addr: String,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Nothing>(
        cx,
        Arc::new(|Args { addr }| {
            println!("shutting down node {:?}", addr);
            let node = node_connection(&addr).map_err(|e| anyhow!("error connecting to node {}: {}", addr, e))?;
            let key = default_private_key().map_err(|e| anyhow!("error getting default key: {}", e))?;
            run_ft(async move { node.connect(&key).await?.shutdown().await })
                .map_err(|e| anyhow!("error shutting down node: {}", e))?;
            Ok(Nothing {})
        }),
    );
    Ok(ud)
}
