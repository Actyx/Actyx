pub use crate::api::events::service::EventService;
use crate::util::formats::NodeCycleCount;
use ax_sdk::{AppId, Timestamp};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::AppMode;

#[derive(Debug, Clone, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
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

#[cfg(test)]
mod bearer_token_tests {
    use ax_sdk::{app_id, Timestamp};
    use std::time::Duration;

    use super::{AppMode, BearerToken};

    #[test]
    fn bearer_token_is_expired() {
        let token = BearerToken {
            created: Timestamp::now() - Duration::from_secs(2),
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        assert!(token.is_expired());

        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 300,
            app_mode: AppMode::Signed,
        };
        assert!(!token.is_expired());
    }

    #[test]
    fn bearer_token_expiration() {
        let now = Timestamp::now();
        let token = BearerToken {
            created: now,
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        assert_eq!(token.expiration(), now + Duration::from_secs(token.validity as u64));
    }

    #[test]
    fn bearer_round_trip() {
        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        let json = serde_json::to_string(&token).unwrap();
        let round_tripped = serde_json::from_str(&json).unwrap();
        assert_eq!(token, round_tripped);
    }

    #[test]
    fn bearer_wire_format() {
        let json = r#"{
              "created": 1619769229417484,
              "appId": "app-id",
              "cycles": 42,
              "appVersion": "1.4.2",
              "validity": 10,
              "appMode": "signed"
            }"#;
        let des: BearerToken = serde_json::from_str(json).unwrap();
        let token = BearerToken {
            created: Timestamp::from(1619769229417484),
            app_id: app_id!("app-id"),
            cycles: 42.into(),
            app_version: "1.4.2".into(),
            validity: 10,
            app_mode: AppMode::Signed,
        };
        assert_eq!(des, token);
    }
}
