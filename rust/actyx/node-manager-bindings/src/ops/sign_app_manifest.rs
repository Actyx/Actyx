use anyhow::anyhow;
use ax_core::certs::app_manifest_signer::sign_manifest_from_files;
use neon::{
    context::{Context, FunctionContext},
    result::JsResult,
    types::JsUndefined,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::util::run_task;
use ax_sdk::types::AppManifest;
use futures::FutureExt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Args {
    path_to_certificate: PathBuf,
    path_to_manifest: PathBuf,
}
pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, AppManifest>(
        cx,
        Box::new(
            |mut _tx,
             Args {
                 path_to_certificate,
                 path_to_manifest,
             }| {
                async move {
                    sign_manifest_from_files(path_to_certificate, path_to_manifest)
                        .map_err(|e| anyhow!("error signing manifest: {}", e))
                }
                .boxed()
            },
        ),
    )?;
    Ok(ud)
}
