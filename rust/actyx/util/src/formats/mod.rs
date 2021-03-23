use derive_more::{AsRef, Display, From, Into};
use serde::Deserialize;

pub mod admin_protocol;
pub mod errors;
pub mod logs;

pub use admin_protocol::*;
pub use errors::*;
pub use logs::*;

pub const ACTYXOS_ID: &str = "com.actyx.os";

#[derive(Deserialize, PartialEq, Clone, Debug, From, Into, AsRef, Display)]
pub struct NodeName(pub String);

pub mod node_error_context {
    use super::*;
    #[derive(Debug, Display)]
    pub struct BindingFailed(pub u16);
    #[derive(Debug, Display)]
    pub struct Component(pub String);
}

#[macro_export]
/// Wrapper around `panic!` making sure that the passed in arg evalutes to
/// `Arc<anyhow::Error>`. This is to be used in conjunction with the panic hook
/// handler installed inside the `node` crate in order to pass an error object
/// via a panic without information loss ([`node::util::init_panic_hook`]).
macro_rules! ax_panic {
    ($x:expr) => {
        let y: Arc<anyhow::Error> = $x;
        panic!(y);
    };
}
