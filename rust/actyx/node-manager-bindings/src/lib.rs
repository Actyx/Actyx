mod consts;
mod ops;
mod types;
mod util;

use crate::util::{create_default_private_key, default_private_key};
use axlib::{
    node_connection::{mk_swarm, Task},
    util::{formats::ActyxOSCode, setup_logger},
};
use futures::channel::mpsc;
use neon::prelude::*;
use tokio::runtime::Runtime;

struct Ctx {
    rt: Runtime,
    tx: mpsc::Sender<Task>,
}
impl Finalize for Ctx {}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    let rt = Runtime::new().or_else(|e| cx.throw_error(format!("{e}\n\n")))?;
    let key = default_private_key()
        .or_else(|e| {
            if e.code() == ActyxOSCode::ERR_USER_UNAUTHENTICATED {
                create_default_private_key()
            } else {
                Err(e)
            }
        })
        .or_else(|e| cx.throw_error(format!("{e}\n\n")))?;
    let (task, tx) = rt
        .block_on(mk_swarm(key))
        .or_else(|e| cx.throw_error(format!("{e}\n\n")))?;
    rt.spawn(task);
    let ctx = cx.boxed(Ctx { rt, tx });
    cx.export_value("_ctx", ctx)?;

    setup_logger();

    cx.export_function("connect", ops::connect::js)?;
    cx.export_function("getNodeDetails", ops::get_node_details::js)?;
    cx.export_function("createUserKeyPair", ops::create_user_key_pair::js)?;
    cx.export_function("setSettings", ops::set_settings::js)?;
    cx.export_function("generateSwarmKey", ops::generate_swarm_key::js)?;
    cx.export_function("signAppManifest", ops::sign_app_manifest::js)?;
    cx.export_function("shutdown", ops::shutdown_node::js)?;
    cx.export_function("query", ops::query::js)?;
    cx.export_function("publish", ops::publish::js)?;
    cx.export_function("onDisconnect", ops::on_disconnect::js)?;
    cx.export_function("deleteTopic", ops::delete_topic::js)?;
    cx.export_function("getTopicList", ops::get_topic_list::js)?;
    Ok(())
}
