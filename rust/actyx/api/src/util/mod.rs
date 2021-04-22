pub mod filters;
pub mod hyper_serve;

use std::time::Duration;

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
pub struct BearerToken {
    /// when it was created
    pub created: Timestamp,
    /// for whom
    pub app_id: AppId,
    /// restart cycle count of Actyx node that created it
    pub cycles: u64,
    /// Actyx version
    pub version: String,
    /// intended validity in seconds
    pub validity: u32,
    /// Actyx trial mode?,
    pub trial_mode: bool,
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
    use std::time::Duration;

    use actyxos_sdk::{app_id, Timestamp};

    use super::BearerToken;

    #[test]
    fn bearer_token_is_expired() {
        let token = BearerToken {
            created: Timestamp::now() - Duration::from_secs(2),
            app_id: app_id!("app id"),
            cycles: 0,
            version: "1.0.0".into(),
            validity: 1,
            trial_mode: false,
        };
        assert_eq!(token.is_expired(), true);

        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("app id"),
            cycles: 0,
            version: "1.0.0".into(),
            validity: 300,
            trial_mode: false,
        };
        assert_eq!(token.is_expired(), false);
    }

    #[test]
    fn bearer_token_expiration() {
        let now = Timestamp::now();
        let token = BearerToken {
            created: now,
            app_id: app_id!("app id"),
            cycles: 0,
            version: "1.0.0".into(),
            validity: 1,
            trial_mode: false,
        };
        assert_eq!(token.expiration(), now + Duration::from_secs(token.validity as u64));
    }
}
