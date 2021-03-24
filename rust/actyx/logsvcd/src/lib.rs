#![deny(clippy::future_not_send)]

#[cfg(target_os = "android")]
mod android_logcat;
mod error;
pub mod logging_sink;
mod storage_query;
pub use logging_sink::*;
mod storage;
mod sync;
use crossbeam::channel;
pub use formats::*;
pub use storage::*;
pub use sync::*;
mod formats;
use actyxos_sdk::NodeId;
use std::path::Path;

#[derive(Debug)]
pub struct LogConfig {
    _static: StaticConfig,
    dynamic: channel::Receiver<DynamicConfig>,
}

#[derive(Debug)]
pub struct StaticConfig {
    /// Where to store the db.
    pub db_path: String,
    /// Days to keep log records
    retention_days: usize,
    /// Max size of the db [bytes]
    retention_size: usize,
    /// Filter passed to logcat
    logcat_filter: String,
}
#[derive(Debug, PartialEq, Clone)]
pub struct DynamicConfig {
    /// NodeId from the node - will be added to the logs.
    pub node_id: NodeId,
    /// Serial number of this device - will be added to the logs.
    pub node_name: String,
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            db_path: "logsvcd.sqlite".to_string(),
            retention_days: 7,
            retention_size: 50_000_000,
            logcat_filter: "*".into(),
        }
    }
}
impl LogConfig {
    pub fn with_dir<P: AsRef<Path>>(p: P, dynamic: channel::Receiver<DynamicConfig>) -> Self {
        let _static = StaticConfig {
            db_path: p.as_ref().join("logsvcd.sqlite").display().to_string(),
            ..Default::default()
        };

        Self { _static, dynamic }
    }
}
