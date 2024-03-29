use crate::{types::Nothing, util::run_task};
use ax_core::private_key::generate_key;
use futures::FutureExt;
use neon::{
    context::{Context, FunctionContext},
    result::JsResult,
    types::JsUndefined,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Res {
    swarm_key: String,
}

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Nothing, Res>(
        cx,
        Box::new(|_, _| {
            async move {
                Ok(Res {
                    swarm_key: generate_key(),
                })
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
