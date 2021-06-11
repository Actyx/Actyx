use crate::{
    types::Nothing,
    util::{node_request, run_task},
};
use anyhow::anyhow;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use util::formats::AdminRequest;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    addr: String,
    settings: serde_json::Value,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Nothing>(
        cx,
        Arc::new(|Args { addr, settings }| {
            node_request(
                &addr,
                AdminRequest::SettingsSet {
                    scope: settings::Scope {
                        tokens: vec!["com.actyx".to_string()],
                    },
                    json: settings,
                    ignore_errors: false,
                },
            )
            .map_err(|e| anyhow!("error settings settings: {}", e))
            .map(|_| Nothing {})
        }),
    );
    Ok(ud)
}
