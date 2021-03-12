use actyxos_sdk::{tagged::AppId, TimeStamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct BearerToken {
    /// when it was created
    pub created: TimeStamp,
    /// for whom
    pub app_id: AppId,
    /// restart cycle count of ActyxOS node that created it
    pub cycles: u64,
    /// ActyxOS version
    pub version: String,
    /// intended validity in seconds
    pub validity: u32,
}
