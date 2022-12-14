use crate::Ctx;
use anyhow::Result;
use axlib::{
    node_connection::Task,
    private_key::{AxPrivateKey, DEFAULT_PRIVATE_KEY_FILE_NAME},
};
use futures::{channel::mpsc::Sender, future::BoxFuture};
use neon::{
    context::{Context, FunctionContext},
    object::Object,
    result::NeonResult,
    types::{JsBox, JsFunction, JsString},
};
use serde::{de::DeserializeOwned, Serialize};
use std::convert::TryFrom;
use util::formats::{ActyxOSCode, ActyxOSResult};

pub fn to_stringified<Se: Serialize>(s: Se) -> Result<String> {
    Ok(serde_json::to_string(&s)?)
}

pub fn from_stringified<'a, De: DeserializeOwned>(cx: &mut impl Context<'a>, str: String) -> NeonResult<De> {
    match serde_json::from_str::<De>(str.as_str()) {
        Ok(v) => Ok(v),
        Err(err) => cx.throw_error(err.to_string()),
    }
}

pub fn default_private_key() -> ActyxOSResult<AxPrivateKey> {
    AxPrivateKey::try_from(&None)
}

pub fn create_default_private_key() -> ActyxOSResult<AxPrivateKey> {
    let path = AxPrivateKey::get_and_create_default_user_identity_dir()?.join(DEFAULT_PRIVATE_KEY_FILE_NAME);
    if path.exists() {
        return Err(ActyxOSCode::ERR_FILE_EXISTS.with_message(path.display().to_string()));
    }
    eprintln!("writing fresh default keypair to {}", path.display());
    let key = AxPrivateKey::generate();
    key.to_file(path)?;
    eprintln!("keypair written");
    Ok(key)
}

#[allow(clippy::type_complexity)]
pub fn run_task<I: serde::de::DeserializeOwned + Sync + Send + 'static, O: serde::Serialize + Sync + Send + 'static>(
    mut cx: FunctionContext,
    executor: Box<dyn Fn(Sender<Task>, I) -> BoxFuture<'static, Result<O>> + Send + 'static>,
) -> NeonResult<()> {
    let ctx = cx
        .this()
        .get(&mut cx, "_ctx")?
        .downcast_or_throw::<JsBox<Ctx>, _>(&mut cx)?;
    let json_input = cx.argument::<JsString>(0).map(|h| h.value(&mut cx))?;
    let input: I = from_stringified(&mut cx, json_input)?;

    let callback = cx.argument::<JsFunction>(1)?;
    let callback = callback.root(&mut cx);
    let queue = cx.channel();
    let tx = ctx.tx.clone();
    ctx.rt.spawn(async move {
        let f = executor(tx, input);
        let res = f.await;
        queue.send(move |mut cx| {
            let callback = callback.into_inner(&mut cx);
            let undef = cx.undefined();
            let empty_str = cx.string("");
            match res.and_then(to_stringified) {
                Err(err) => {
                    let stringified_err = cx.string(err.to_string());
                    callback.call(&mut cx, undef, vec![stringified_err, empty_str])?;
                }
                Ok(stringified_res) => {
                    let stringified_res = cx.string(stringified_res);
                    callback.call(&mut cx, undef, vec![empty_str, stringified_res])?;
                }
            };
            Ok(())
        });
        Ok::<(), ()>(())
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::types::Nothing;

    use super::*;
    #[test]
    fn test_to_stringified() -> Result<()> {
        assert_eq!(to_stringified(Nothing {})?, "{}");
        Ok(())
    }
}
