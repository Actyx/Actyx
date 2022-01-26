pub mod admin_protocol;
pub mod banyan_protocol;
pub mod errors;
pub mod events_protocol;
pub mod logs;

pub mod os_arch;

pub use admin_protocol::*;
pub use errors::*;
pub use logs::*;

use derive_more::{AsRef, Display, From, Into};
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

/// Keeps track of how many times a node was restarted
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, From)]
pub struct NodeCycleCount(u64);

#[derive(Deserialize, PartialEq, Clone, Debug, From, Into, AsRef, Display)]
pub struct NodeName(pub String);

#[derive(Debug, Display)]
pub enum NodeErrorContext {
    #[display(fmt = "Bind failed on port {} for {}", addr, component)]
    BindFailed { addr: Multiaddr, component: String },
}

#[macro_export]
/// Wrapper around `panic!` making sure that the passed in arg evalutes to
/// `anyhow::Error`. This is to be used in conjunction with the panic hook
/// handler installed inside the `node` crate in order to pass an error object
/// via a panic without information loss ([`node::util::init_panic_hook`]).
macro_rules! ax_panic {
    ($x:expr) => {
        let y: anyhow::Error = $x;
        ::std::panic::panic_any(::std::sync::Arc::new(y));
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;

    #[derive(Clone, Debug, Display, PartialEq)]
    #[display(fmt = "Whatever")]
    struct MyCustomError {
        the_answer: usize,
    }

    #[test]
    fn panic_err_roundtrip() {
        let custom_error = MyCustomError { the_answer: 42 };
        let error_from_panic = std::panic::catch_unwind(|| {
            std::panic::panic_any(Arc::new(anyhow::anyhow!(custom_error.clone())));
        })
        .unwrap_err();

        let extracted_anyhow = error_from_panic.downcast_ref::<Arc<anyhow::Error>>().unwrap();
        let extracted_error = extracted_anyhow.downcast_ref::<MyCustomError>().unwrap();
        assert_eq!(custom_error, *extracted_error);
    }
}
