mod app_domain;
mod app_license;
mod app_manifest;
mod developer_certificate;
mod signature;

pub use app_domain::AppDomain;
pub use app_license::{AppLicense, AppLicenseType, Expiring, RequesterInfo, SignedAppLicense};
pub use app_manifest::{app_manifest_signer, AppManifestSignature, AppManifestSignatureProps};
pub use developer_certificate::{DeveloperCertificate, DeveloperCertificateInput, ManifestDeveloperCertificate};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        certs::{developer_certificate::DeveloperCertificateInput, signature::Signature},
        crypto::PrivateKey,
    };
    use actyx_sdk::{app_id, AppManifest};
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
        let manifest_from_json: AppManifest = serde_json::from_str(json).unwrap();

        let signature = AppManifestSignature::new(
            Signature::new(
                &AppManifestSignatureProps {
                    app_id: app_id!("my.examples.test-app"),
                    display_name: "display name".to_owned(),
                    version: "v0.0.1".to_owned(),
                },
                dev_private_key(),
            )
            .unwrap(),
            ManifestDeveloperCertificate::new(
                DeveloperCertificateInput::new(dev_private_key().into(), app_domains()),
                ax_private_key(),
            )
            .unwrap(),
        );
        let signature_string: String = signature.try_into().unwrap();
        let constructed_manifest = AppManifest::signed(
            app_id!("my.examples.test-app"),
            "display name".into(),
            "v0.0.1".into(),
            signature_string,
        );

        assert_eq!(manifest_from_json, constructed_manifest);
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
            AppManifest::trial(app_id!("com.example.test-app"), "display name".into(), "v0.0.1".into()).unwrap()
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
            "Trial app id needs to start with 'com.example.'. Got 'my.examples.test-app'."
        );
    }
}
