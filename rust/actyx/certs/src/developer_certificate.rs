use actyx_sdk::AppId;
use crypto::{PrivateKey, PublicKey};
use derive_more::{Display, Error};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{app_domain::AppDomain, signature::Signature};

#[derive(Debug, Display, Error)]
#[display(fmt = "AppId '{}' is not allowed in app_domains '{:?}'", app_id, app_domains)]
pub struct InvalidAppId {
    app_id: AppId,
    app_domains: Vec<AppDomain>,
}

impl InvalidAppId {
    pub fn new(app_id: AppId, app_domains: Vec<AppDomain>) -> Self {
        Self { app_id, app_domains }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperCertificateInput {
    dev_pubkey: PublicKey,
    app_domains: Vec<AppDomain>,
}

impl DeveloperCertificateInput {
    pub fn new(dev_pubkey: PublicKey, app_domains: Vec<AppDomain>) -> Self {
        Self {
            dev_pubkey,
            app_domains,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestDeveloperCertificate {
    #[serde(flatten)]
    input: DeveloperCertificateInput,
    ax_signature: Signature,
}

impl ManifestDeveloperCertificate {
    // this is done by oliver@actyx.io using the signing utility
    pub fn new(input: DeveloperCertificateInput, ax_privkey: PrivateKey) -> anyhow::Result<Self> {
        let ax_signature = Signature::new(&input, ax_privkey)?;
        Ok(Self { input, ax_signature })
    }

    pub fn validate(&self, ax_public_key: PublicKey) -> anyhow::Result<()> {
        self.ax_signature.verify(&self.input, ax_public_key)
    }

    pub fn validate_app_id(&self, app_id: &AppId) -> anyhow::Result<()> {
        match self
            .input
            .app_domains
            .clone()
            .into_iter()
            .any(|x| x.is_app_id_allowed(app_id))
        {
            true => Ok(()),
            false => Err(InvalidAppId::new(app_id.clone(), self.input.app_domains.clone()).into()),
        }
    }

    pub fn dev_public_key(&self) -> PublicKey {
        self.input.dev_pubkey
    }
}

fn serialize_dev_private_key<S: Serializer>(x: &PrivateKey, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&x.to_string())
}

fn deserialize_dev_private_key<'de, D: Deserializer<'de>>(d: D) -> Result<PrivateKey, D::Error> {
    let s = <String>::deserialize(d)?;
    s.parse::<PrivateKey>().map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperCertificate {
    #[serde(serialize_with = "serialize_dev_private_key")]
    #[serde(deserialize_with = "deserialize_dev_private_key")]
    dev_privkey: PrivateKey,
    #[serde(flatten)]
    manifest_dev_cert: ManifestDeveloperCertificate,
}

impl DeveloperCertificate {
    pub fn new(dev_privkey: PrivateKey, app_domains: Vec<AppDomain>, ax_privkey: PrivateKey) -> anyhow::Result<Self> {
        let input = DeveloperCertificateInput::new(dev_privkey.into(), app_domains);
        let manifest_dev_cert = ManifestDeveloperCertificate::new(input, ax_privkey)?;
        Ok(Self {
            dev_privkey,
            manifest_dev_cert,
        })
    }

    pub fn private_key(&self) -> PrivateKey {
        self.dev_privkey
    }

    pub fn manifest_dev_cert(&self) -> ManifestDeveloperCertificate {
        self.manifest_dev_cert.clone()
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::app_id;
    use crypto::{PrivateKey, PublicKey};

    use crate::developer_certificate::{AppDomain, DeveloperCertificate, DeveloperCertificateInput, InvalidAppId};

    use super::ManifestDeveloperCertificate;
    struct TestFixture {
        ax_private_key: PrivateKey,
        ax_public_key: PublicKey,
        dev_public_key: PublicKey,
        dev_private_key: PrivateKey,
        app_domains: Vec<AppDomain>,
        manifest_dev_cert: serde_json::Value,
        dev_cert: serde_json::Value,
    }

    fn setup() -> TestFixture {
        let ax_signature = "zJXkEd/PgZ7dpE3d5CsBRibGV0QpOYpHGkwfWRDTSnrY5wnmX3zbxL605NGc+xnNzJxz2jjw7UEzcONPkEstCw==";
        let app_domain: AppDomain = "com.actyx.*".parse().unwrap();

        let ax_private_key: PrivateKey = "0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=".parse().unwrap();
        let ax_public_key = PublicKey::from(ax_private_key);

        let dev_private_key: PrivateKey = "0BKoAIaJ1z3AM+hqJiquSoPvEMbnIeznncmZxo24j5SY=".parse().unwrap();
        let dev_public_key = PublicKey::from(dev_private_key);

        let manifest_dev_cert = serde_json::json!({
            "devPubkey": dev_public_key.to_string(),
            "appDomains": [app_domain.to_string()],
            "axSignature": ax_signature
        });

        let dev_cert = serde_json::json!({
            "devPrivkey": dev_private_key.to_string(),
            "devPubkey": dev_public_key.to_string(),
            "appDomains": [app_domain.to_string()],
            "axSignature": ax_signature
        });

        TestFixture {
            ax_private_key,
            ax_public_key,
            dev_private_key,
            dev_public_key,
            app_domains: vec![app_domain],
            manifest_dev_cert,
            dev_cert,
        }
    }

    #[test]
    fn create_and_serialize() {
        let x = setup();
        let input = DeveloperCertificateInput::new(x.dev_public_key, x.app_domains);
        let dev_cert = ManifestDeveloperCertificate::new(input, x.ax_private_key).unwrap();
        let serialized = serde_json::to_value(&dev_cert).unwrap();
        assert_eq!(serialized, x.manifest_dev_cert);
    }

    #[test]
    fn deserialize_and_validate() {
        let x = setup();
        let dev_cert: ManifestDeveloperCertificate = serde_json::from_value(x.manifest_dev_cert).unwrap();
        let ok_result = dev_cert.validate(x.ax_public_key);
        assert!(matches!(ok_result, Ok(())), "valid signature");
    }

    #[test]
    fn deserialize_and_fail_signature_validation_when_key_is_wrong() {
        let x = setup();
        let dev_cert: ManifestDeveloperCertificate = serde_json::from_value(x.manifest_dev_cert).unwrap();
        let err_result = dev_cert.validate(x.dev_public_key);
        assert!(
            matches!(err_result, Err(anyhow::Error { .. })),
            "invalid signature or key"
        );
    }

    #[test]
    fn validate_app_id_failure() {
        let x = setup();
        let dev_cert: ManifestDeveloperCertificate = serde_json::from_value(x.manifest_dev_cert).unwrap();
        let err = dev_cert.validate_app_id(&app_id!("com.example.test-app")).unwrap_err();
        err.downcast_ref::<InvalidAppId>()
            .unwrap_or_else(|| panic!("Found wrong error: {}", err));
        assert_eq!(
            err.to_string(),
            "AppId \'com.example.test-app\' is not allowed in app_domains \'[AppDomain(\"com.actyx.*\")]\'"
        );
    }

    #[test]
    fn validate_app_id_success() {
        let x = setup();
        let dev_cert: ManifestDeveloperCertificate = serde_json::from_value(x.manifest_dev_cert).unwrap();
        let result = dev_cert.validate_app_id(&app_id!("com.actyx.test-app"));
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn validate_app_id_success_2() {
        let x = setup();
        let input = DeveloperCertificateInput {
            dev_pubkey: x.dev_public_key,
            app_domains: vec!["com.example.*".parse().unwrap(), "com.actyx.*".parse().unwrap()],
        };
        let dev_cert = ManifestDeveloperCertificate::new(input, x.ax_private_key).unwrap();
        let result = dev_cert.validate_app_id(&app_id!("com.actyx.test-app"));
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn should_fail_validation_for_tempered_props() {
        let x = setup();
        vec![("0siGG7GXH", "0sigg7gxh"), ("com.actyx", "com.example")]
            .into_iter()
            .for_each(|(from, to)| {
                let dev_cert: ManifestDeveloperCertificate =
                    serde_json::from_str(&x.manifest_dev_cert.to_string().replace(from, to)).unwrap();
                let result = dev_cert.validate(x.dev_public_key).unwrap_err();
                assert_eq!(result.to_string(), "Invalid signature for provided input.");
            });
    }

    #[test]
    fn create_and_serialize_developer_certificate() {
        let x = setup();
        let dev_cert = DeveloperCertificate::new(x.dev_private_key, x.app_domains, x.ax_private_key).unwrap();
        let serialized = serde_json::to_value(&dev_cert).unwrap();
        assert_eq!(serialized, x.dev_cert);
    }

    #[test]
    fn deserialize_developer_certificate() {
        let x = setup();
        let dev_cert: DeveloperCertificate = serde_json::from_value(x.dev_cert).unwrap();
        let expected_dev_cert = DeveloperCertificate::new(x.dev_private_key, x.app_domains, x.ax_private_key).unwrap();
        assert_eq!(dev_cert, expected_dev_cert);
    }

    #[test]
    fn deserialize_developer_certificate_to_manifest_developer_cert() {
        let x = setup();
        let dev_cert: ManifestDeveloperCertificate = serde_json::from_value(x.dev_cert).unwrap();
        let input = DeveloperCertificateInput::new(x.dev_public_key, x.app_domains);
        let expected_dev_cert = ManifestDeveloperCertificate::new(input, x.ax_private_key).unwrap();
        assert_eq!(dev_cert, expected_dev_cert);
    }
}
