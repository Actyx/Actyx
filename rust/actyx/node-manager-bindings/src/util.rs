use anyhow::Result;
use axlib::{node_connection::connect, private_key::AxPrivateKey};
use neon::context::Context;
use neon::context::FunctionContext;
use neon::object::Object;
use neon::types::JsFunction;
use neon::types::JsString;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::sync::Arc;
use std::{convert::TryFrom, str::FromStr};
use tokio::runtime::Runtime;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse};

pub fn to_stringified<Se: Serialize>(s: Se) -> Result<String> {
    Ok(serde_json::to_string(&s)?)
}

pub fn from_stringified<De: DeserializeOwned>(str: String) -> Result<De> {
    match serde_json::from_str::<De>(str.as_str()) {
        Ok(v) => Ok(v),
        Err(err) => Err(anyhow::anyhow!("{}", err)),
    }
}

// This may seem like a strange function, but in this case it is quite useful.
// The reason is that many of the Actyx internal functions are built around
// futures to run in multi-threaded/async environments (which makes lots of
// sense). In this case though, the functions are executed async by Node.js,
// meaning we don't need to provide an async runtime ourselves and it is
// completely fine to just block on the current thread (which is already async).
pub fn run_ft<T, F: Future<Output = ActyxOSResult<T>>>(future: F) -> ActyxOSResult<T> {
    let rt = Runtime::new()
        .map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_INTERNAL_ERROR, format!("error running future: {}", e)))?;
    rt.block_on(future)
}

pub fn default_private_key() -> ActyxOSResult<AxPrivateKey> {
    AxPrivateKey::try_from(&None)
}

pub fn run_task<I: serde::de::DeserializeOwned + Sync + Send + 'static, O: serde::Serialize + Sync + Send + 'static>(
    mut cx: FunctionContext,
    executor: Arc<dyn Fn(I) -> Result<O> + Sync + Send>,
) {
    let json_input = match cx.argument::<JsString>(0).map(|h| h.value(&mut cx)) {
        Ok(str) => str,
        Err(err) => {
            // Panic turns into JS exception (throws)
            panic!("error getting task json_input argument {}", err.to_string());
        }
    };
    let input: I = match from_stringified(json_input) {
        Ok(i) => i,
        Err(err) => {
            panic!("error decoding json_input argument {}", err.to_string());
        }
    };

    let callback = match cx.argument::<JsFunction>(1) {
        Ok(cb) => cb,
        Err(err) => {
            panic!("error getting callback argument {}", err.to_string());
        }
    };
    let callback = callback.root(&mut cx);
    let queue = cx.channel();
    std::thread::spawn(move || {
        let res = executor(input);
        queue.send(move |mut cx| {
            let callback = callback.into_inner(&mut cx);
            let undef = cx.undefined();
            let empty_str = cx.string("");
            let call_res = match res.and_then(to_stringified) {
                Err(err) => {
                    let stringified_err = cx.string(err.to_string());
                    callback.call(&mut cx, undef, vec![stringified_err, empty_str])
                }
                Ok(stringified_res) => {
                    let stringified_res = cx.string(stringified_res);
                    callback.call(&mut cx, undef, vec![empty_str, stringified_res])
                }
            };
            if let Err(err) = call_res {
                panic!("error calling task callback {}", err.to_string());
            }
            Ok(())
        });
        Ok::<(), ()>(())
    });
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
