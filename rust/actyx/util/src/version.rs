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
    /// Returns the version associated with ACTYX_VERSION (compile time env var)
    pub fn get() -> NodeVersion {
        NodeVersion {
            profile: env!("AX_PROFILE").to_string(),
            target: OsArch::current().into(),
            version: env!("AX_VERSION").to_string(),
            git_hash: env!("AX_GIT_HASH").to_string(),
        }
    }

    /// Returns the version associated with ACTYX_VERSION_CLI (compile time env var)
    pub fn get_cli() -> NodeVersion {
        NodeVersion {
            profile: env!("AX_PROFILE").to_string(),
            target: OsArch::current().into(),
            version: env!("AX_CLI_VERSION").to_string(),
            git_hash: env!("AX_GIT_HASH").to_string(),
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
