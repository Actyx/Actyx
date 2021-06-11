use anyhow::anyhow;
use axlib::cmd::apps::{create_signed_app_manifest, SignOpts};
use certs::SignedAppManifest;
use neon::{
    context::{Context, FunctionContext},
    result::JsResult,
    types::JsUndefined,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

use crate::util::run_task;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Args {
    path_to_certificate: PathBuf,
    path_to_manifest: PathBuf,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, SignedAppManifest>(
        cx,
        Arc::new(
            |Args {
                 path_to_certificate,
                 path_to_manifest,
             }| {
                create_signed_app_manifest(SignOpts {
                    path_to_certificate,
                    path_to_manifest,
                })
                .map_err(|e| anyhow!("error signing manifest: {}", e))
            },
        ),
    );
    Ok(ud)
}
