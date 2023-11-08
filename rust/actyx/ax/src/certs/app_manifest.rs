use crate::certs::{developer_certificate::ManifestDeveloperCertificate, signature::Signature};
use crate::crypto::{PrivateKey, PublicKey};
use actyx_sdk::AppId;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppManifestSignatureProps {
    pub app_id: AppId,
    pub display_name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppManifestSignature {
    // this is zero (for now)
    sig_version: u8,
    dev_signature: Signature,
    #[serde(flatten)]
    dev_cert: ManifestDeveloperCertificate,
}

impl AppManifestSignature {
    pub(crate) fn new(dev_signature: Signature, dev_cert: ManifestDeveloperCertificate) -> Self {
        Self {
            sig_version: 0,
            dev_signature,
            dev_cert,
        }
    }
}

fn serialize_signature<S: Serializer>(sig: &Option<AppManifestSignature>, s: S) -> Result<S::Ok, S::Error> {
    let bytes = serde_cbor::to_vec(sig).map_err(serde::ser::Error::custom)?;
    s.serialize_str(&base64::encode(bytes))
}

impl FromStr for AppManifestSignature {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = base64::decode(s).map_err(|_| "failed to base64 decode app manifest signature")?;
        serde_cbor::from_slice::<AppManifestSignature>(&data)
            .map_err(|_| "failed to deserialize to app manifest signature")
    }
}

fn deserialize_signature<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<AppManifestSignature>, D::Error> {
    let s = String::deserialize(deserializer)?;
    let sig = s.parse::<AppManifestSignature>().map_err(D::Error::custom)?;
    Ok(Some(sig))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppManifestIo {
    pub app_id: AppId,
    pub display_name: String,
    pub version: String,
    #[serde(serialize_with = "serialize_signature", skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_signature", default)]
    pub signature: Option<AppManifestSignature>,
}

impl From<AppManifest> for AppManifestIo {
    fn from(value: AppManifest) -> Self {
        value.0
    }
}

impl TryFrom<AppManifestIo> for AppManifest {
    type Error = String;

    fn try_from(value: AppManifestIo) -> Result<Self, Self::Error> {
        if value.signature.is_none() && !value.app_id.starts_with("com.example.") {
            Err(format!(
                "Trial app id needs to start with 'com.example.'. Got '{}'.",
                value.app_id
            ))
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(into = "AppManifestIo", try_from = "AppManifestIo")]
pub struct AppManifest(AppManifestIo);

impl AppManifest {
    pub fn sign(
        app_id: AppId,
        display_name: String,
        version: String,
        dev_privkey: PrivateKey,
        dev_cert: ManifestDeveloperCertificate,
    ) -> anyhow::Result<Self> {
        dev_cert.validate_app_id(&app_id)?;
        let hash_input = AppManifestSignatureProps {
            app_id: app_id.clone(),
            display_name: display_name.clone(),
            version: version.clone(),
        };
        let dev_signature = Signature::new(&hash_input, dev_privkey)?;
        let manifest_signature = AppManifestSignature::new(dev_signature, dev_cert);
        Ok(Self(AppManifestIo {
            app_id,
            display_name,
            version,
            signature: Some(manifest_signature),
        }))
    }

    pub fn signed(app_id: AppId, display_name: String, version: String, signature: AppManifestSignature) -> Self {
        Self(AppManifestIo {
            app_id,
            display_name,
            version,
            signature: Some(signature),
        })
    }

    pub fn trial(app_id: AppId, display_name: String, version: String) -> anyhow::Result<Self> {
        if !app_id.starts_with("com.example.") {
            anyhow::bail!("Trial app id needs to start with 'com.example.'. Got '{}'.", app_id);
        }
        Ok(Self(AppManifestIo {
            app_id,
            display_name,
            version,
            signature: None,
        }))
    }

    pub fn app_id(&self) -> AppId {
        self.0.app_id.clone()
    }

    pub fn display_name(&self) -> &str {
        &self.0.display_name
    }

    pub fn version(&self) -> &str {
        &self.0.version
    }

    pub fn is_signed(&self) -> bool {
        self.0.signature.is_some()
    }

    pub fn validate(&self, ax_public_key: &PublicKey) -> anyhow::Result<()> {
        if let Some(signature) = &self.0.signature {
            // Check signature on the dev cert
            signature
                .dev_cert
                .validate(ax_public_key)
                .map_err(|x| anyhow::Error::msg(format!("Failed to validate developer certificate. {}", x)))?;
            // Check app id matches allowed domains
            signature.dev_cert.validate_app_id(&self.0.app_id)?;
            // Check manifest hash signature
            let hash_input = AppManifestSignatureProps {
                app_id: self.0.app_id.clone(),
                display_name: self.0.display_name.clone(),
                version: self.0.version.clone(),
            };
            signature
                .dev_signature
                .verify(&hash_input, &signature.dev_cert.dev_public_key())
                .map_err(|x| anyhow::Error::msg(format!("Failed to validate app manifest. {}", x)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::crypto::{PrivateKey, PublicKey};
    use actyx_sdk::app_id;

    use crate::certs::{
        app_domain::AppDomain,
        developer_certificate::{DeveloperCertificateInput, ManifestDeveloperCertificate},
        signature::Signature,
    };

    use super::{AppManifest, AppManifestSignature, AppManifestSignatureProps};

    struct TestFixture {
        ax_public_key: PublicKey,
        dev_private_key: PrivateKey,
        dev_cert: ManifestDeveloperCertificate,
        sig_props: AppManifestSignatureProps,
        serialized_manifest: serde_json::Value,
    }

    fn setup() -> TestFixture {
        let ax_private_key = PrivateKey::from_str("0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=").unwrap();
        let dev_private_key = PrivateKey::from_str("0BKoAIaJ1z3AM+hqJiquSoPvEMbnIeznncmZxo24j5SY=").unwrap();
        let dev_public_key: PublicKey = dev_private_key.into();
        let app_domains: Vec<AppDomain> = vec!["com.actyx.*".parse().unwrap()];
        let dev_cert = DeveloperCertificateInput::new(dev_public_key, app_domains);
        let dev_cert = ManifestDeveloperCertificate::new(dev_cert, ax_private_key).unwrap();
        let serialized_manifest = serde_json::json!({
            "appId":"com.actyx.test-app",
            "displayName":"display name",
            "version":"version 0",
            "signature":"v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYWWNuNmlpWXllcFBKeEN6L3hEY3JkQklHcGlxYVRkQVBaakVuRnIzL2xBdGgzaXRzNU9PaTFORjU4M2xFcTJuSVJwOWtIZ1BFSTViTFNPQlFxN2lNQkE9PWlkZXZQdWJrZXl4LTBzaUdHN0dYSGpGaG5oRldya3RiaVZ2Vjgyb1dxTUVzdDBiOVVjWjZYRWd3PWphcHBEb21haW5zgWtjb20uYWN0eXguKmtheFNpZ25hdHVyZXhYekpYa0VkL1BnWjdkcEUzZDVDc0JSaWJHVjBRcE9ZcEhHa3dmV1JEVFNuclk1d25tWDN6YnhMNjA1TkdjK3huTnpKeHoyamp3N1VFemNPTlBrRXN0Q3c9Pf8="
        });
        TestFixture {
            ax_public_key: ax_private_key.into(),
            dev_private_key,
            dev_cert,
            sig_props: AppManifestSignatureProps {
                app_id: app_id!("com.actyx.test-app"),
                display_name: "display name".into(),
                version: "version 0".into(),
            },
            serialized_manifest,
        }
    }

    #[test]
    fn serialize() {
        let x = setup();
        let manifest = AppManifest::sign(
            x.sig_props.app_id,
            x.sig_props.display_name,
            x.sig_props.version,
            x.dev_private_key,
            x.dev_cert,
        )
        .unwrap();
        let serialized = serde_json::to_value(&manifest).unwrap();
        assert_eq!(serialized, x.serialized_manifest);
    }

    #[test]
    fn deserialize_and_eq() {
        let x = setup();
        let manifest = serde_json::from_value::<AppManifest>(x.serialized_manifest).unwrap();
        let expected = AppManifest::sign(
            x.sig_props.app_id,
            x.sig_props.display_name,
            x.sig_props.version,
            x.dev_private_key,
            x.dev_cert,
        )
        .unwrap();
        assert_eq!(manifest, expected);
    }

    #[test]
    fn validate() {
        let x = setup();
        let manifest = serde_json::from_value::<AppManifest>(x.serialized_manifest).unwrap();
        let result = manifest.validate(&x.ax_public_key);
        assert!(matches!(result, Ok(())), "valid signature");
    }

    #[test]
    fn should_fail_validation_when_using_wrong_ax_public_key() {
        let x = setup();
        let manifest = serde_json::from_value::<AppManifest>(x.serialized_manifest).unwrap();
        let result = manifest.validate(&PrivateKey::generate().into()).unwrap_err();
        assert_eq!(
            result.to_string(),
            "Failed to validate developer certificate. Invalid signature for provided input."
        );
    }

    #[test]
    fn should_fail_validation_for_tampered_props() {
        let x = setup();
        vec![
            ("com.actyx.test-app", "com.actyx.another-test-app"),
            ("display name", "some name"),
            ("version 0", "1"),
        ]
        .into_iter()
        .for_each(|(from, to)| {
            let manifest: AppManifest =
                serde_json::from_str(&x.serialized_manifest.to_string().replace(from, to)).unwrap();
            let result = manifest.validate(&PrivateKey::generate().into()).unwrap_err();
            assert_eq!(
                result.to_string(),
                "Failed to validate developer certificate. Invalid signature for provided input."
            );
        });
    }

    #[test]
    fn test_app_manifest_signature_version_is_0() {
        let private = PrivateKey::generate();
        let dev_signature = Signature::new(&[1; 3], private).unwrap();
        // create whatever dev_cert
        let input = DeveloperCertificateInput::new(private.into(), Vec::new());
        let dev_cert = ManifestDeveloperCertificate::new(input, private).unwrap();
        let signature = AppManifestSignature::new(dev_signature, dev_cert);
        assert_eq!(signature.sig_version, 0);
    }

    #[test]
    fn should_succeed_creating_and_serializing_trial_manifest() {
        let manifest =
            AppManifest::trial(app_id!("com.example.test-app"), "display name".into(), "v0.0.1".into()).unwrap();
        let serialized = serde_json::to_string(&manifest).unwrap();
        assert_eq!(
            serialized,
            r#"{"appId":"com.example.test-app","displayName":"display name","version":"v0.0.1"}"#
        )
    }

    #[test]
    fn should_fail_creating_trial_manifest() {
        let err =
            AppManifest::trial(app_id!("com.actyx.test-app"), "display name".into(), "v0.0.1".into()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Trial app id needs to start with 'com.example.'. Got 'com.actyx.test-app'."
        );
    }
}
