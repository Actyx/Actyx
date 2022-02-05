use crate::types::Nothing;
use crate::util::run_task;
use axlib::cmd::swarms::keygen::generate_key;
use futures::FutureExt;
use neon::prelude::*;
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
