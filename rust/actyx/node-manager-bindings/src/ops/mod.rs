macro_rules! filter {
    ($req:path => $res:path) => {
        |res| match res {
            $res(r) => Ok(r),
            r => ax_core::util::formats::ax_err(
                ax_core::util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
                format!("{} returned mismatched response: {:?}", stringify!($req), r),
            ),
        }
    };
}

pub(crate) mod connect;
pub(crate) mod create_user_key_pair;
pub(crate) mod delete_topic;
pub(crate) mod generate_swarm_key;
pub(crate) mod get_node_details;
pub(crate) mod get_topic_list;
pub(crate) mod on_disconnect;
pub(crate) mod publish;
pub(crate) mod query;
pub(crate) mod set_settings;
pub(crate) mod shutdown_node;
pub(crate) mod sign_app_manifest;
