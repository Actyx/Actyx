use crate::Ctx;
use axlib::node_connection::Task;
use futures::SinkExt;
use neon::{
    prelude::{Context, FunctionContext, Object},
    result::JsResult,
    types::{JsBox, JsFunction, JsUndefined},
};
use tokio::sync::{mpsc, oneshot};

pub fn js(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let ctx = cx
        .this()
        .get(&mut cx, "_ctx")?
        .downcast_or_throw::<JsBox<Ctx>, _>(&mut cx)?;

    let callback = cx.argument::<JsFunction>(0)?;
    let mut callback = callback.root(&mut cx);
    let queue = cx.channel();
    let mut task = ctx.tx.clone();
    ctx.rt.spawn(async move {
        let (tx, mut rx) = mpsc::unbounded_channel();
        if task.feed(Task::OnDisconnect(tx)).await.is_err() {
            queue.send(move |mut cx| {
                callback.drop(&mut cx);
                Ok(())
            });
            return Ok(());
        }

        while let Some(peer) = rx.recv().await {
            let (cb_tx, cb_rx) = oneshot::channel();
            queue.send(move |mut cx| {
                let peer = cx.string(&peer.to_string());
                let undef = cx.undefined();
                callback.to_inner(&mut cx).call(&mut cx, undef, vec![peer])?;
                cb_tx.send(callback).ok();
                Ok(())
            });
            callback = cb_rx.await.unwrap();
        }
        Ok::<_, ()>(())
    });

    Ok(cx.undefined())
}
