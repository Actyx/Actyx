mod consts;
mod ops;
mod types;
mod util;

use self::util::default_private_key;
use ::util::setup_logger;
use axlib::node_connection::{mk_swarm, Task};
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
    let rt = Runtime::new().unwrap();
    let (task, tx) = rt.block_on(mk_swarm(default_private_key().unwrap())).unwrap();
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
    cx.export_function("onDisconnect", ops::on_disconnect::js)?;
    Ok(())
}
