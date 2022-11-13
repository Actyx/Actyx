use crate::{types::Nothing, util::run_task};
use axlib::node_connection::{request_single, Task};
use futures::FutureExt;
use neon::prelude::*;
use serde::{Deserialize, Serialize};
use util::formats::{AdminRequest, AdminResponse};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    peer: String,
    settings: serde_json::Value,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Nothing>(
        cx,
        Box::new(|mut tx, Args { peer, settings }| {
            async move {
                let peer_id = peer.parse()?;
                request_single(
                    &mut tx,
                    move |tx| {
                        Task::Admin(
                            peer_id,
                            AdminRequest::SettingsSet {
                                scope: settings::Scope {
                                    tokens: vec!["com.actyx".to_string()],
                                },
                                json: settings,
                                ignore_errors: false,
                            },
                            tx,
                        )
                    },
                    filter!(AdminRequest::SettingsSet => AdminResponse::SettingsSetResponse),
                )
                .await?;
                Ok(Nothing {})
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
