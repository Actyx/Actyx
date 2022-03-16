use actyx_sdk::AppId;
use crypto::{PrivateKey, PublicKey};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use crate::{developer_certificate::ManifestDeveloperCertificate, signature::Signature};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppManifestSignatureProps {
    pub app_id: AppId,
    pub display_name: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AppManifestSignature {
    // this is zero (for now)
    sig_version: u8,
    dev_signature: Signature,
    #[serde(flatten)]
    dev_cert: ManifestDeveloperCertificate,
}

impl AppManifestSignature {
    fn new(dev_signature: Signature, dev_cert: ManifestDeveloperCertificate) -> Self {
        Self {
            sig_version: 0,
            dev_signature,
            dev_cert,
        }
    }
}

fn serialize_signature<S: Serializer>(sig: &AppManifestSignature, s: S) -> Result<S::Ok, S::Error> {
    let bytes = serde_cbor::to_vec(sig).map_err(serde::ser::Error::custom)?;
    s.serialize_str(&base64::encode(bytes))
}

fn deserialize_signature<'de, D: Deserializer<'de>>(d: D) -> Result<AppManifestSignature, D::Error> {
    let s = <String>::deserialize(d)?;
    let data = base64::decode(s).map_err(|_| D::Error::custom("failed to base64 decode app manifest signature"))?;
    serde_cbor::from_slice::<AppManifestSignature>(&data)
        .map_err(|_| D::Error::custom("failed to deserialize to app manifest signature"))
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignedAppManifest {
    pub app_id: AppId,
    pub display_name: String,
    pub version: String,
    #[serde(serialize_with = "serialize_signature")]
    #[serde(deserialize_with = "deserialize_signature")]
    pub signature: AppManifestSignature,
}

impl SignedAppManifest {
    pub fn new(
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
        Ok(SignedAppManifest {
            app_id,
            display_name,
            version,
            signature: manifest_signature,
        })
    }

    pub fn get_app_id(&self) -> AppId {
        self.app_id.clone()
    }

    pub fn validate(&self, ax_public_key: &PublicKey) -> anyhow::Result<()> {
        // Check signature on the dev cert
        self.signature
            .dev_cert
            .validate(ax_public_key)
            .map_err(|x| anyhow::Error::msg(format!("Failed to validate developer certificate. {}", x)))?;
        // Check app id matches allowed domains
        self.signature.dev_cert.validate_app_id(&self.app_id)?;
        // Check manifest hash signature
        let hash_input = AppManifestSignatureProps {
            app_id: self.app_id.clone(),
            display_name: self.display_name.clone(),
            version: self.version.clone(),
        };
        self.signature
            .dev_signature
            .verify(&hash_input, &self.signature.dev_cert.dev_public_key())
            .map_err(|x| anyhow::Error::msg(format!("Failed to validate app manifest. {}", x)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use actyx_sdk::app_id;
    use crypto::{PrivateKey, PublicKey};

    use crate::{
        app_domain::AppDomain,
        developer_certificate::{DeveloperCertificateInput, ManifestDeveloperCertificate},
        signature::Signature,
    };

    use super::{AppManifestSignature, AppManifestSignatureProps, SignedAppManifest};

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
        let manifest = SignedAppManifest::new(
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
        let manifest: SignedAppManifest = serde_json::from_value(x.serialized_manifest).unwrap();
        let expected = SignedAppManifest::new(
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
        let manifest: SignedAppManifest = serde_json::from_value(x.serialized_manifest).unwrap();
        let result = manifest.validate(&x.ax_public_key);
        assert!(matches!(result, Ok(())), "valid signature");
    }

    #[test]
    fn should_fail_validation_when_using_wrong_ax_public_key() {
        let x = setup();
        let manifest: SignedAppManifest = serde_json::from_value(x.serialized_manifest).unwrap();
        let result = manifest.validate(&PrivateKey::generate().into()).unwrap_err();
        assert_eq!(
            result.to_string(),
            "Failed to validate developer certificate. Invalid signature for provided input."
        );
    }

    #[test]
    fn should_fail_validation_for_tempered_props() {
        let x = setup();
        vec![
            ("com.actyx.test-app", "com.actyx.another-test-app"),
            ("display name", "some name"),
            ("version 0", "1"),
        ]
        .into_iter()
        .for_each(|(from, to)| {
            let manifest: SignedAppManifest =
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
}
