use std::sync::Arc;

use axlib::cmd::swarms::keygen::generate_key;
use neon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::Nothing;
use crate::util::run_task;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Res {
    swarm_key: String,
}

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Nothing, Res>(
        cx,
        Arc::new(|_| {
            Ok(Res {
                swarm_key: generate_key(),
            })
        }),
    );
    Ok(ud)
}
