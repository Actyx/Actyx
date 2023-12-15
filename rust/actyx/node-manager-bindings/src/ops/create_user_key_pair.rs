use anyhow::bail;
use ax_core::private_key::{AxPrivateKey, DEFAULT_PRIVATE_KEY_FILE_NAME};
use neon::{
    context::{Context, FunctionContext},
    result::JsResult,
    types::JsUndefined,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::util::run_task;
use futures::FutureExt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Args {
    private_key_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Res {
    private_key_path: String,
    public_key: String,
    public_key_path: String,
}

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ud = cx.undefined();
    run_task::<Args, Res>(
        cx,
        Box::new(|_tx, Args { private_key_path }| {
            async move {
                let private_key_path = private_key_path.unwrap_or(
                    AxPrivateKey::get_and_create_default_user_identity_dir()?.join(DEFAULT_PRIVATE_KEY_FILE_NAME),
                );
                if private_key_path.exists() {
                    bail!(
                        "File {} already exits in the specified path. Specify a different file name or path.",
                        private_key_path.display()
                    );
                }
                let key = AxPrivateKey::generate();
                let (private_key_path, public_key_path) = key.to_file(&private_key_path)?;
                let public_key = key.to_string();
                Ok(Res {
                    private_key_path: private_key_path.to_string_lossy().into_owned(),
                    public_key_path: public_key_path.to_string_lossy().into_owned(),
                    public_key,
                })
            }
            .boxed()
        }),
    )?;
    Ok(ud)
}
