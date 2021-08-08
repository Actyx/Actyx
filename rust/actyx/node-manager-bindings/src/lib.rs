use neon::prelude::*;
mod ops;
mod types;
mod util;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("getNodeDetails", ops::get_node_details::js)?;
    cx.export_function("createUserKeyPair", ops::create_user_key_pair::js)?;
    cx.export_function("setSettings", ops::set_settings::js)?;
    cx.export_function("generateSwarmKey", ops::generate_swarm_key::js)?;
    cx.export_function("signAppManifest", ops::sign_app_manifest::js)?;
    cx.export_function("shutdown", ops::shutdown_node::js)?;
    cx.export_function("query", ops::query::js)?;
    Ok(())
}
