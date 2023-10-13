mod app_domain;
mod app_license;
mod developer_certificate;
mod signature;
mod signed_app_manifest;
mod trial_app_manifest;

pub use app_domain::AppDomain;
pub use app_license::{AppLicense, AppLicenseType, Expiring, RequesterInfo, SignedAppLicense};
pub use developer_certificate::{DeveloperCertificate, DeveloperCertificateInput, ManifestDeveloperCertificate};
pub use signed_app_manifest::{AppManifestSignatureProps, SignedAppManifest};
pub use trial_app_manifest::TrialAppManifest;

use crate::signed_app_manifest::AppManifestSignature;
use actyx_sdk::AppId;
use serde::{
    de::{Error, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AppManifest {
    // NB! Signed needs to come before Trial, due to how serde deserialize untagged enums
    Signed(SignedAppManifest),
    Trial(TrialAppManifest),
}

impl<'de> Deserialize<'de> for AppManifest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = AppManifest;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut app_id = None;
                let mut version = None;
                let mut display_name = None;
                let mut signature = None;
                while let Some((k, v)) = map.next_entry::<String, String>()? {
                    match &*k {
                        "appId" => app_id = Some(AppId::try_from(&*v).map_err(A::Error::custom)?),
                        "version" => version = Some(v),
                        "displayName" => display_name = Some(v),
                        "signature" => signature = Some(v.parse::<AppManifestSignature>().map_err(A::Error::custom)?),
                        _ => {}
                    }
                }
                let Some(app_id) = app_id else {
                    return Err(A::Error::missing_field("appId"));
                };
                let Some(version) = version else {
                    return Err(A::Error::missing_field("version"));
                };
                let Some(display_name) = display_name else {
                    return Err(A::Error::missing_field("displayName"));
                };
                if let Some(signature) = signature {
                    Ok(AppManifest::Signed(SignedAppManifest {
                        app_id,
                        display_name,
                        version,
                        signature,
                    }))
                } else {
                    Ok(AppManifest::Trial(
                        TrialAppManifest::new(app_id, display_name, version).map_err(A::Error::custom)?,
                    ))
                }
            }
        }
        deserializer.deserialize_struct("manifest", &["appId", "version", "displayName", "signature"], V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{developer_certificate::DeveloperCertificateInput, signature::Signature};
    use actyx_sdk::app_id;
    use crypto::PrivateKey;
    use std::str::FromStr;

    fn ax_private_key() -> PrivateKey {
        PrivateKey::from_str("0mIt0wGJTC6Ux2CKhxO6j94u9U8MMDgtJGpTag1T9iKU=").unwrap()
    }

    fn dev_private_key() -> PrivateKey {
        PrivateKey::from_str("0rZ0PD8kRI4yFPbOpiy6Esi4tFltTaL1lEOaMULUiXVc=").unwrap()
    }

    fn app_domains() -> Vec<AppDomain> {
        vec![AppDomain::from_str("my.examples.*").unwrap()]
    }

    #[test]
    fn should_deserialize_signed_app_manifest() {
        let json = r#"{
            "appId": "my.examples.test-app",
            "displayName": "display name",
            "version": "v0.0.1",
            "signature": "v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYeXZ4R0haYjRubEVZNmEyNkR3U3Y4Q2pqQXNTVDBqd2pxcExjMzFHalFDVFIyZEhEVFBUQ1BmMHRSMnlWWnBMY3BMeGorRVpDV2RLZUxoZjZxbG1ZRFE9PWlkZXZQdWJrZXl4LTBBQzhrZ3ZOUXVSdXp6bVY1Z3hTeGVyanJTeE1BQ25pSytjY3V0UGpZdHg0PWphcHBEb21haW5zgW1teS5leGFtcGxlcy4qa2F4U2lnbmF0dXJleFhYRkN5WVoyeDVkVFFaTUlPYVhJa25IcVR2Y0hkVHBxMUkvZENBYXZmclJmRmNaZDNmSVQ3dHlLbzNFWjVxN0pUNlhEM3JycldKdkJVTUdsN2RzU3RCUT09/w=="
        }"#;
        let manifest: AppManifest = serde_json::from_str(json).unwrap();
        assert_eq!(
            manifest,
            AppManifest::Signed(SignedAppManifest {
                app_id: app_id!("my.examples.test-app"),
                display_name: "display name".into(),
                version: "v0.0.1".into(),
                signature: AppManifestSignature::new(
                    Signature::new(
                        &AppManifestSignatureProps {
                            app_id: app_id!("my.examples.test-app"),
                            display_name: "display name".to_owned(),
                            version: "v0.0.1".to_owned()
                        },
                        dev_private_key()
                    )
                    .unwrap(),
                    ManifestDeveloperCertificate::new(
                        DeveloperCertificateInput::new(dev_private_key().into(), app_domains()),
                        ax_private_key()
                    )
                    .unwrap()
                )
            })
        );
    }

    #[test]
    fn should_deserialize_trial_app_manifest() {
        let json = r#"{
            "appId": "com.example.test-app",
            "displayName": "display name",
            "version": "v0.0.1"
        }"#;
        let manifest: AppManifest = serde_json::from_str(json).unwrap();
        assert_eq!(
            manifest,
            AppManifest::Trial(
                TrialAppManifest::new(app_id!("com.example.test-app"), "display name".into(), "v0.0.1".into()).unwrap()
            )
        );
    }

    #[test]
    fn error_trial_not_com_example() {
        let json = r#"{
            "appId": "my.examples.test-app",
            "displayName": "display name",
            "version": "v0.0.1"
        }"#;
        let error = serde_json::from_str::<AppManifest>(json).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Trial app id needs to start with 'com.example.'. Got 'my.examples.test-app'. at line 5 column 9"
        );
    }
}
