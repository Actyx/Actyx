use std::str::FromStr;

use actyx_sdk::AppId;
use anyhow::Context;
use chrono::{DateTime, Utc};
use crypto::{PrivateKey, PublicKey};
use serde::{Deserialize, Serialize};

use crate::signature::Signature;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct RequesterInfo {
    email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Expiring {
    app_id: AppId,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum AppLicenseType {
    Expiring(Expiring),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct AppLicense {
    // this is zero for now
    license_version: u8,
    license_type: AppLicenseType,
    /// when it was created
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignedAppLicense {
    #[serde(flatten)]
    license: AppLicense,
    signature: Signature,
    requester: RequesterInfo,
}

impl SignedAppLicense {
    pub fn new(
        ax_private_key: PrivateKey,
        email: String,
        app_id: AppId,
        expires_at: DateTime<Utc>,
        created_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<Self> {
        let license = AppLicense {
            license_version: 0,
            license_type: AppLicenseType::Expiring(Expiring { app_id, expires_at }),
            created_at: created_at.unwrap_or_else(Utc::now),
        };
        let signature = Signature::new(&license, ax_private_key)?;
        Ok(Self {
            license,
            signature,
            requester: RequesterInfo { email },
        })
    }

    pub fn validate(&self, ax_public_key: &PublicKey) -> anyhow::Result<()> {
        self.signature.verify(&self.license, ax_public_key)
    }

    pub fn to_base64(&self) -> anyhow::Result<String> {
        let bytes = serde_cbor::to_vec(&self)?;
        Ok(base64::encode(bytes))
    }
}

impl FromStr for SignedAppLicense {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = base64::decode(s).context("Failed to base64 decode app license")?;
        serde_cbor::from_slice::<SignedAppLicense>(&data).context("Failed to deserialize to app license")
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::{app_id, AppId};
    use chrono::{DateTime, TimeZone, Utc};
    use crypto::{PrivateKey, PublicKey};

    use crate::{app_license::SignedAppLicense, signature::InvalidSignature};

    struct TestFixture {
        ax_private_key: PrivateKey,
        ax_public_key: PublicKey,
        serialized_license: serde_json::Value,
        created_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        app_id: AppId,
        email: String,
    }

    fn setup() -> TestFixture {
        let ax_private_key: PrivateKey = "0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=".parse().unwrap();
        let ax_public_key = PublicKey::from(ax_private_key);
        let app_id = app_id!("com.actyx.auth-test");
        let created_at = Utc.ymd(1970, 1, 1).and_hms(0, 1, 1);
        let expires_at = Utc.ymd(1971, 1, 1).and_hms(0, 1, 1);
        let email: String = "customer@example.com".into();

        let serialized_license = serde_json::json!({
            "licenseVersion": 0,
            "licenseType": {
                "expiring":{
                    "appId": app_id,
                    "expiresAt": expires_at
                }
            },
            "createdAt": created_at,
            "signature": "AAdRwU8Q6eorKcCwJ15OKx9eOCI163w0XpLZGZCOQiCYFebDupYAmiM9XlowCal9uKnIhQzY3IJ6FGTlKI1+DQ==",
            "requester": {
                "email": email,
            }
        });

        TestFixture {
            ax_private_key,
            ax_public_key,
            serialized_license,
            created_at,
            expires_at,
            app_id,
            email,
        }
    }

    #[test]
    fn create_and_serialize() {
        let x = setup();
        let license =
            SignedAppLicense::new(x.ax_private_key, x.email, x.app_id, x.expires_at, Some(x.created_at)).unwrap();
        let serialized = serde_json::to_value(&license).unwrap();
        assert_eq!(serialized, x.serialized_license);
    }

    #[test]
    fn deserialize_and_validate() {
        let x = setup();
        let license: SignedAppLicense = serde_json::from_value(x.serialized_license).unwrap();
        let ok_result = license.validate(&x.ax_public_key);
        assert!(matches!(ok_result, Ok(())));
    }

    #[test]
    fn deserialize_and_fail_signature_validation_when_using_not_matching_public_key() {
        let x = setup();
        let license: SignedAppLicense = serde_json::from_value(x.serialized_license).unwrap();
        let err = license.validate(&PrivateKey::generate().into()).unwrap_err();
        err.downcast_ref::<InvalidSignature>()
            .unwrap_or_else(|| panic!("Found wrong error: {}", err));
        assert_eq!(err.to_string(), "Invalid signature for provided input.");
    }

    #[test]
    fn to_base64_and_back() {
        let x = setup();
        let license =
            SignedAppLicense::new(x.ax_private_key, x.email, x.app_id, x.expires_at, Some(x.created_at)).unwrap();
        let serialized = license.to_base64().unwrap();
        let expected = "v25saWNlbnNlVmVyc2lvbgBrbGljZW5zZVR5cGWhaGV4cGlyaW5nomVhcHBJZHNjb20uYWN0eXguYXV0aC10ZXN0aWV4cGlyZXNBdHQxOTcxLTAxLTAxVDAwOjAxOjAxWmljcmVhdGVkQXR0MTk3MC0wMS0wMVQwMDowMTowMVppc2lnbmF0dXJleFhBQWRSd1U4UTZlb3JLY0N3SjE1T0t4OWVPQ0kxNjN3MFhwTFpHWkNPUWlDWUZlYkR1cFlBbWlNOVhsb3dDYWw5dUtuSWhRelkzSUo2RkdUbEtJMStEUT09aXJlcXVlc3RlcqFlZW1haWx0Y3VzdG9tZXJAZXhhbXBsZS5jb23/";
        assert_eq!(serialized, expected);

        let deserialized: SignedAppLicense = expected.parse().unwrap();
        assert_eq!(deserialized, license);
    }
}
