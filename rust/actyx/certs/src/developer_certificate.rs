use actyx_sdk::AppId;
use crypto::{PrivateKey, PublicKey};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

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
pub struct DeveloperCertificate {
    #[serde(flatten)]
    input: DeveloperCertificateInput,
    ax_signature: Signature,
}

impl DeveloperCertificate {
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

#[cfg(test)]
mod tests {
    use actyx_sdk::app_id;
    use crypto::{PrivateKey, PublicKey};

    use crate::developer_certificate::{AppDomain, DeveloperCertificateInput, InvalidAppId};

    use super::DeveloperCertificate;
    struct TestFixture {
        ax_private_key: PrivateKey,
        ax_public_key: PublicKey,
        dev_public_key: PublicKey,
        app_domains: Vec<AppDomain>,
        dev_cert: serde_json::Value,
    }
    fn setup() -> TestFixture {
        let ax_private_key: PrivateKey = "0WBFFicIHbivRZXAlO7tPs7rCX6s7u2OIMJ2mx9nwg0w=".parse().unwrap();
        let dev_private_key: PrivateKey = "0BKoAIaJ1z3AM+hqJiquSoPvEMbnIeznncmZxo24j5SY=".parse().unwrap();
        let dev_cert = serde_json::json!({
            "devPubkey": "0siGG7GXHjFhnhFWrktbiVvV82oWqMEst0b9UcZ6XEgw=",
            "appDomains": ["com.actyx.*"],
            "axSignature": "zJXkEd/PgZ7dpE3d5CsBRibGV0QpOYpHGkwfWRDTSnrY5wnmX3zbxL605NGc+xnNzJxz2jjw7UEzcONPkEstCw=="
        });
        TestFixture {
            ax_private_key,
            ax_public_key: ax_private_key.into(),
            dev_public_key: dev_private_key.into(),
            app_domains: vec!["com.actyx.*".parse().unwrap()],
            dev_cert,
        }
    }

    #[test]
    fn create_and_serialize() {
        let x = setup();
        let input = DeveloperCertificateInput::new(x.dev_public_key, x.app_domains);
        let dev_cert = DeveloperCertificate::new(input, x.ax_private_key).unwrap();
        let serialized = serde_json::to_value(&dev_cert).unwrap();
        assert_eq!(serialized, x.dev_cert);
    }

    #[test]
    fn deserialize_and_validate() {
        let x = setup();
        let dev_cert: DeveloperCertificate = serde_json::from_value(x.dev_cert).unwrap();
        let ok_result = dev_cert.validate(x.ax_public_key);
        assert!(matches!(ok_result, Ok(())), "valid signature");
    }

    #[test]
    fn deserialize_and_fail_signature_validation_when_key_is_wrong() {
        let x = setup();
        let dev_cert: DeveloperCertificate = serde_json::from_value(x.dev_cert).unwrap();
        let err_result = dev_cert.validate(x.dev_public_key);
        assert!(
            matches!(err_result, Err(anyhow::Error { .. })),
            "invalid signature or key"
        );
    }

    #[test]
    fn validate_app_id_failure() {
        let x = setup();
        let dev_cert: DeveloperCertificate = serde_json::from_value(x.dev_cert).unwrap();
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
        let dev_cert: DeveloperCertificate = serde_json::from_value(x.dev_cert).unwrap();
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
        let dev_cert = DeveloperCertificate::new(input, x.ax_private_key).unwrap();
        let result = dev_cert.validate_app_id(&app_id!("com.actyx.test-app"));
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn should_fail_validation_for_tempered_props() {
        let x = setup();
        vec![("0siGG7GXH", "0sigg7gxh"), ("com.actyx", "com.example")]
            .into_iter()
            .for_each(|(from, to)| {
                let dev_cert: DeveloperCertificate =
                    serde_json::from_str(&x.dev_cert.to_string().replace(from, to)).unwrap();
                let result = dev_cert.validate(x.dev_public_key).unwrap_err();
                assert_eq!(result.to_string(), "Invalid signature for provided input.");
            });
    }
}
