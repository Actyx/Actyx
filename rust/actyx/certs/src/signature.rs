use std::{convert::TryInto, str::FromStr};

use anyhow::Context;
use crypto::{KeyPair, PrivateKey, PublicKey};
use derive_more::{Display, Error};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Display, Error)]
#[display(fmt = "Invalid signature for provided input.")]
pub struct InvalidSignature();

fn deserialize_signature<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
    let s = <String>::deserialize(d)?;
    s.parse()
        .map(|x: Signature| x.0)
        .map_err(|x| D::Error::custom(x.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Signature(#[serde(deserialize_with = "deserialize_signature")] [u8; 64]);

impl Serialize for Signature {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base64::encode(self.0))
    }
}

impl FromStr for Signature {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = base64::decode(s).context("error base64 decoding Signature")?;
        let result: [u8; 64] = data.try_into().map_err(|x: Vec<u8>| {
            let context = format!("Expected a Vec of length 64 but it was {}", x.len());
            anyhow::Error::msg(context)
        })?;
        Ok(Self(result))
    }
}

impl Signature {
    fn serialize_canonically<T: Serialize>(input: &T) -> anyhow::Result<Vec<u8>> {
        serde_cbor::to_vec(input).context("error serializing input to cbor")
    }

    pub fn new<T: Serialize>(input: &T, key: PrivateKey) -> anyhow::Result<Self> {
        let bytes = Signature::serialize_canonically(input)?;
        let key = KeyPair::from(key);
        let signature = key.sign(&bytes);
        Ok(Self(signature))
    }

    pub fn verify<T: Serialize>(&self, input: &T, key: &PublicKey) -> anyhow::Result<()> {
        let hash = Signature::serialize_canonically(input)?;
        match key.verify(&hash, self.0.as_ref()) {
            true => Ok(()),
            false => Err(InvalidSignature().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crypto::{PrivateKey, PublicKey};
    use serde::Serialize;

    use crate::signature::{InvalidSignature, Signature};

    #[derive(Clone, Serialize)]
    struct TestStruct {
        data: String,
    }

    struct TestFixture {
        private_key: PrivateKey,
        public_key: PublicKey,
        test_data: TestStruct,
        sig_as_string: String,
    }

    fn setup() -> TestFixture {
        let private_key = PrivateKey::from_str("0vPsv8jygvJ2o1l3hu+FGauc+OA53QFUQO6e3+hcp8uc=").unwrap();
        let public_key: PublicKey = private_key.into();
        let sig_as_string =
            "mdpSNyWrAXeZQpWc0rWcqUJQhfFDiLF6Xh2rm354QFmzP/uXbTl6mZeaId5RoGZx8U9U1s13vZ796kUNYhajBA==".to_owned();
        let test_data = TestStruct {
            data: "some data".into(),
        };
        TestFixture {
            private_key,
            public_key,
            test_data,
            sig_as_string,
        }
    }

    #[test]
    fn has_stable_hash() {
        let x = setup();
        let signature = Signature::new(&x.test_data, x.private_key).unwrap();
        assert_eq!(signature.to_string(), x.sig_as_string);
    }

    #[test]
    fn equal_by_value() {
        let x = setup();
        assert_eq!(
            Signature::new(&x.test_data, x.private_key).unwrap(),
            Signature::new(&x.test_data.clone(), x.private_key).unwrap()
        );
    }

    #[test]
    fn signature_validate() {
        let x = setup();
        let signature: Signature = x.sig_as_string.parse().unwrap();
        let ok_result = signature.verify(&x.test_data, &x.public_key);
        assert!(matches!(ok_result, Ok(())), "valid signature");
    }

    #[test]
    fn fail_validation_when_signature_is_tempered() {
        let x = setup();
        let tempered_signature = x.sig_as_string.replace("mdpSN", "mdpZZ");
        let signature: Signature = tempered_signature.parse().unwrap();
        let err = signature.verify(&x.test_data, &x.public_key).unwrap_err();
        err.downcast_ref::<InvalidSignature>()
            .unwrap_or_else(|| panic!("Found wrong error: {}", err));
        assert_eq!(err.to_string(), "Invalid signature for provided input.");
    }

    #[test]
    fn fail_validation_when_signature_is_for_another_payload() {
        let x = setup();
        let signature: Signature = x.sig_as_string.parse().unwrap();
        let test_data2 = TestStruct {
            data: "some data 2".into(),
        };
        let err_result = signature.verify(&test_data2, &x.public_key);
        assert!(matches!(err_result, Err(anyhow::Error { .. })));
    }

    #[test]
    fn serialize_deserialize() {
        let x = setup();
        let signature = Signature::new(&x.test_data, x.private_key).unwrap();
        let serialized = serde_json::to_string(&signature).unwrap();
        assert_eq!(serialized, format!("\"{}\"", x.sig_as_string),);

        let deserialized: Signature = serde_json::from_str(&serialized).unwrap();
        assert_eq!(signature, deserialized);
    }
}
