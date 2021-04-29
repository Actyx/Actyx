use serde::{Deserialize, Serialize};

use crate::formats::os_arch::OsArch;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeVersion {
    pub profile: String,
    pub target: String,
    pub version: String,
    pub git_hash: String,
}

impl NodeVersion {
    pub fn get() -> NodeVersion {
        NodeVersion {
            profile: env!("PROFILE").to_string(),
            target: OsArch::current().into(),
            version: env!("VERSION").to_string(),
            git_hash: env!("GIT_HASH").to_string(),
        }
    }
}

impl std::fmt::Display for NodeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}-{}-{}-{}",
            self.version, self.git_hash, self.target, self.profile
        ))
    }
}
