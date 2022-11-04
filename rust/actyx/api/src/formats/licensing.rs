use crate::rejections::{ApiError, UnauthorizedReason};
use actyx_sdk::AppId;
use certs::{AppLicenseType, Expiring, SignedAppLicense};
use chrono::Utc;
use crypto::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Licensing {
    node: String,
    pub apps: BTreeMap<AppId, String>,
}

impl Licensing {
    pub fn new(node: String, apps: BTreeMap<AppId, String>) -> Self {
        Self { node, apps }
    }

    pub fn is_node_licensed(&self, ax_public_key: &PublicKey) -> Result<bool, ApiError> {
        if self.node == "development" {
            return Ok(false);
        }
        let license = self
            .node
            .parse::<SignedAppLicense>()
            .map_err(|_| ApiError::NodeUnauthorized {
                reason: UnauthorizedReason::MalformedLicense,
            })?;
        license
            .validate(ax_public_key)
            .map_err(|_| ApiError::NodeUnauthorized {
                reason: UnauthorizedReason::InvalidSignature,
            })?;
        match license.license.license_type {
            AppLicenseType::Expiring(Expiring { app_id, expires_at }) => {
                if app_id.as_str() != "com.actyx.node" {
                    Err(ApiError::NodeUnauthorized {
                        reason: UnauthorizedReason::WrongSubject,
                    })
                } else if expires_at < Utc::now() {
                    Err(ApiError::NodeUnauthorized {
                        reason: UnauthorizedReason::Expired,
                    })
                } else {
                    Ok(true)
                }
            }
        }
    }

    pub fn app_id_license(&self, app_id: &AppId) -> Option<&String> {
        self.apps.get(app_id)
    }
}

impl Default for Licensing {
    fn default() -> Self {
        Licensing {
            node: "development".into(),
            apps: BTreeMap::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{formats::Licensing, rejections::ApiError, util::get_ax_public_key};
    use std::collections::BTreeMap;

    #[test]
    fn default() {
        let licensing = Licensing::default();
        assert_eq!(licensing.node, "development");
        assert!(licensing.apps.is_empty());
    }

    #[test]
    fn is_node_licensed() {
        let licensing = Licensing::default();
        let ax_key = get_ax_public_key();
        assert!(!licensing.is_node_licensed(&ax_key).unwrap());

        let licensing = Licensing {
            node: "licensed".into(),
            apps: BTreeMap::default(),
        };
        assert_eq!(
            licensing.is_node_licensed(&ax_key).unwrap_err(),
            ApiError::NodeUnauthorized {
                reason: UnauthorizedReason::MalformedLicense
            }
        );
    }
}
