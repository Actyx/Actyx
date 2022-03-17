use serde::{Deserialize, Serialize};

use crate::formats::os_arch::OsArch;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Version {
    major: u8,
    minor: u8,
    patch: u8,
}

impl Version {
    pub fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self { major, minor, patch }
    }
}

impl FromStr for Version {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = if let Some(pos) = s.find('_') {
            s.split_at(pos).0
        } else {
            s
        };
        let mut parts = s.split('.');
        let major = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        let minor = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        let patch = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        Ok(Self { major, minor, patch })
    }
}

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

    pub fn version(&self) -> Option<Version> {
        self.version.parse().ok()
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

impl FromStr for NodeVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('-');
        let version = parts.next().ok_or(())?.to_string();
        let git_hash = parts.next().ok_or(())?.to_string();
        let os = parts.next().ok_or(())?;
        let arch = parts.next().ok_or(())?;
        let target = format!("{}-{}", os, arch);
        let profile = parts.next().ok_or(())?.to_string();
        Ok(Self {
            version,
            git_hash,
            target,
            profile,
        })
    }
}
