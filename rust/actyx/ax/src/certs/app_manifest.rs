use crate::{
    certs::{developer_certificate::ManifestDeveloperCertificate, signature::Signature},
    crypto::{PrivateKey, PublicKey},
};
use actyx_sdk::{AppId, AppManifest};
use serde::{Deserialize, Serialize};
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

impl TryInto<String> for AppManifestSignature {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        let bytes = serde_cbor::to_vec(&self)?;
        Ok(base64::encode(bytes))
    }
}

impl FromStr for AppManifestSignature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = base64::decode(s)?;
        let manifest = serde_cbor::from_slice::<AppManifestSignature>(&data)?;
        Ok(manifest)
    }
}

pub mod app_manifest_signer {
    use super::*;

    pub fn make_signed(
        manifest: &AppManifest,
        dev_privkey: PrivateKey,
        dev_cert: ManifestDeveloperCertificate,
    ) -> anyhow::Result<AppManifest> {
        dev_cert.validate_app_id(&manifest.app_id())?;
        let hash_input = AppManifestSignatureProps {
            app_id: manifest.app_id(),
            display_name: manifest.display_name().into(),
            version: manifest.version().into(),
        };
        let dev_signature = Signature::new(&hash_input, dev_privkey)?;
        let manifest_signature = AppManifestSignature::new(dev_signature, dev_cert);
        let manifest_signature_string: String = manifest_signature.try_into()?;
        let manifest: AppManifest = AppManifest::signed(
            manifest.app_id(),
            manifest.display_name().into(),
            manifest.version().into(),
            manifest_signature_string,
        );
        Ok(manifest)
    }

    pub fn validate(manifest: &AppManifest, ax_public_key: &PublicKey) -> anyhow::Result<()> {
        if let Some(signature) = manifest.signature() {
            let signature = AppManifestSignature::from_str(signature)?;
            // Check signature on the dev cert
            signature
                .dev_cert
                .validate(ax_public_key)
                .map_err(|x| anyhow::Error::msg(format!("Failed to validate developer certificate. {}", x)))?;
            // Check app id matches allowed domains
            let app_id = manifest.app_id();
            signature.dev_cert.validate_app_id(&app_id)?;
            // Check manifest hash signature
            let hash_input = AppManifestSignatureProps {
                app_id,
                display_name: String::from(manifest.display_name()),
                version: String::from(manifest.version()),
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

    use crate::certs::{
        developer_certificate::{DeveloperCertificateInput, ManifestDeveloperCertificate},
        signature::Signature,
    };

    use super::{app_manifest_signer, AppManifest, AppManifestSignature};

    struct TestFixture {
        ax_public_key: PublicKey,
        serialized_manifest: serde_json::Value,
    }

    fn setup() -> TestFixture {
        let ax_private_key = PrivateKey::from_str("0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=").unwrap();
        TestFixture {
            ax_public_key: ax_private_key.into(),
            serialized_manifest: serde_json::json!({
                "appId":"com.actyx.test-app",
                "displayName":"display name",
                "version":"version 0",
                "signature":"v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYWWNuNmlpWXllcFBKeEN6L3hEY3JkQklHcGlxYVRkQVBaakVuRnIzL2xBdGgzaXRzNU9PaTFORjU4M2xFcTJuSVJwOWtIZ1BFSTViTFNPQlFxN2lNQkE9PWlkZXZQdWJrZXl4LTBzaUdHN0dYSGpGaG5oRldya3RiaVZ2Vjgyb1dxTUVzdDBiOVVjWjZYRWd3PWphcHBEb21haW5zgWtjb20uYWN0eXguKmtheFNpZ25hdHVyZXhYekpYa0VkL1BnWjdkcEUzZDVDc0JSaWJHVjBRcE9ZcEhHa3dmV1JEVFNuclk1d25tWDN6YnhMNjA1TkdjK3huTnpKeHoyamp3N1VFemNPTlBrRXN0Q3c9Pf8="
            }),
        }
    }

    #[test]
    fn validate() {
        let x = setup();
        let manifest = serde_json::from_value::<AppManifest>(x.serialized_manifest).unwrap();
        let result = app_manifest_signer::validate(&manifest, &x.ax_public_key);
        assert!(matches!(result, Ok(())), "valid signature");
    }

    #[test]
    fn should_fail_validation_when_using_wrong_ax_public_key() {
        let x = setup();
        let manifest = serde_json::from_value::<AppManifest>(x.serialized_manifest).unwrap();
        let result = app_manifest_signer::validate(&manifest, &PrivateKey::generate().into()).unwrap_err();
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
            let result = app_manifest_signer::validate(&manifest, &PrivateKey::generate().into()).unwrap_err();
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
