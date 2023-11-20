use crate::util::os_arch::OsArch;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
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

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeVersion {
    pub profile: String,
    pub target: String,
    pub version: String,
    pub git_hash: String,
}

const GIT_HASH: &str = match option_env!("AX_GIT_HASH") {
    Some(hash) => hash,
    None => "",
};

lazy_static! {
    pub static ref VERSION: String = NodeVersion::get().to_string();
}

#[cfg(debug_assertions)]
const PROFILE: &str = "debug";
#[cfg(not(debug_assertions))]
const PROFILE: &str = "release";

impl NodeVersion {
    /// Returns the version associated with ACTYX_VERSION (compile time env var)
    pub fn get() -> NodeVersion {
        NodeVersion {
            profile: PROFILE.to_string(),
            target: OsArch::current().into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_hash: GIT_HASH.to_string(),
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
