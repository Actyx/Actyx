//! Bitswap protocol implementation
pub mod behaviour;
pub mod block;
pub mod format;
pub mod peer_stats;
pub mod prefix;
pub mod protocol;
mod util;

pub use behaviour::{Bitswap, BitswapEvent};
pub use block::Block;
pub use protocol::BitswapError;

pub mod codecs {
    // https://github.com/multiformats/multicodec/blob/master/table.csv
    pub const RAW: u64 = 0x55;
    pub const DAG_PROTOBUF: u64 = 0x70;
    pub const DAG_CBOR: u64 = 0x71;
}

mod bitswap_pb {
    include!(concat!(env!("OUT_DIR"), "/bitswap_pb.rs"));
}
