pub mod filters;
pub mod hyper_serve;

use std::time::Duration;

use actyx_util::formats::NodeCycleCount;
use actyxos_sdk::{AppId, Timestamp};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use warp::*;

#[derive(Debug, Display, Deserialize)]
pub struct Token(String);

impl From<String> for Token {
    fn from(x: String) -> Self {
        Self(x)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub enum AppMode {
    Trial,
    Signed,
}

#[derive(Debug, Clone, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct BearerToken {
    /// when it was created
    pub created: Timestamp,
    /// for whom
    pub app_id: AppId,
    /// restart cycle count of Actyx node that created it
    pub cycles: NodeCycleCount,
    /// app version
    pub app_version: String,
    /// intended validity in seconds
    pub validity: u32,
    /// App mode,
    pub app_mode: AppMode,
}

impl BearerToken {
    pub fn is_expired(&self) -> bool {
        Timestamp::now() > self.expiration()
    }

    pub fn expiration(&self) -> Timestamp {
        self.created + Duration::from_secs(self.validity.into())
    }
}

#[derive(Debug, Display)]
pub struct Error(anyhow::Error); // anyhow::Error is sealed so we wrap it
impl std::error::Error for Error {}
impl reject::Reject for Error {}

pub fn reject(err: anyhow::Error) -> Rejection {
    reject::custom(Error(err))
}

pub type Result<T> = std::result::Result<T, Rejection>;

#[cfg(test)]
mod tests {
    use actyxos_sdk::{app_id, Timestamp};
    use std::time::Duration;

    use super::{AppMode, BearerToken};

    #[test]
    fn bearer_token_is_expired() {
        let token = BearerToken {
            created: Timestamp::now() - Duration::from_secs(2),
            app_id: app_id!("app id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        assert_eq!(token.is_expired(), true);

        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("app id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 300,
            app_mode: AppMode::Signed,
        };
        assert_eq!(token.is_expired(), false);
    }

    #[test]
    fn bearer_token_expiration() {
        let now = Timestamp::now();
        let token = BearerToken {
            created: now,
            app_id: app_id!("app id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        assert_eq!(token.expiration(), now + Duration::from_secs(token.validity as u64));
    }
}
