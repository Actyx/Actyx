use axlib::node_connection::Task;
use futures::channel::mpsc;
use neon::prelude::*;
use tokio::runtime::Runtime;
mod consts;
mod ops;
mod types;
mod util;

fn test(mut cx: FunctionContext) -> JsResult<JsString> {
    let ctx = cx
        .this()
        .downcast_or_throw::<JsObject, _>(&mut cx)?
        .get(&mut cx, "_ctx")?
        .downcast_or_throw::<JsBox<Ctx>, _>(&mut cx)?;
    Ok(cx.string(&ctx.s))
}

struct Ctx {
    rt: Runtime,
    tx: mpsc::Sender<Task>,
}
impl Finalize for Ctx {}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    let rt = Runtime::new().unwrap();
    let s = cx.boxed(Ctx { s: "hello".to_owned() });
    cx.export_value("_ctx", s)?;
    cx.export_function("test", test)?;
    // cx.export_function("getNodeDetails", ops::get_node_details::js)?;
    cx.export_function("createUserKeyPair", ops::create_user_key_pair::js)?;
    // cx.export_function("setSettings", ops::set_settings::js)?;
    cx.export_function("generateSwarmKey", ops::generate_swarm_key::js)?;
    cx.export_function("signAppManifest", ops::sign_app_manifest::js)?;
    // cx.export_function("shutdown", ops::shutdown_node::js)?;
    // cx.export_function("query", ops::query::js)?;
    Ok(())
}
