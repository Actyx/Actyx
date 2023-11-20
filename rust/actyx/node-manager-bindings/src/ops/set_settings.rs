use crate::{types::Nothing, util::run_task};
use ax_core::{
    node_connection::{request_single, Task},
    util::formats::{AdminRequest, AdminResponse},
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
    settings: serde_json::Value,
    scope: Vec<String>,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Nothing>(
        cx,
        Box::new(|mut tx, Args { peer, settings, scope }| {
            async move {
                let peer_id = peer.parse()?;
                request_single(
                    &mut tx,
                    move |tx| {
                        let mut tokens = vec!["com.actyx".to_string()];
                        tokens.extend(scope.into_iter());
                        Task::Admin(
                            peer_id,
                            AdminRequest::SettingsSet {
                                scope: ax_core::settings::Scope { tokens },
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
