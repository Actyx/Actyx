use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SerdeFormat {
    Json,
    Cbor,
}

#[derive(Debug, Clone)]
pub struct SerdeFormatParseError(String);

impl std::str::FromStr for SerdeFormat {
    type Err = SerdeFormatParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(serde_json::Value::String(text.into())).map_err(|_| SerdeFormatParseError(text.into()))
    }
}

#[derive(Debug)]
pub enum JsonCborSerializeError {
    JsonError(serde_json::Error),
    CborError(serde_cbor::Error),
}

impl std::fmt::Display for JsonCborSerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for JsonCborSerializeError {}

#[derive(Debug)]
pub struct JsonCborDeserializeError(serde_json::Error, serde_cbor::Error);

impl std::fmt::Display for JsonCborDeserializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unable to deserialize. json error: {} cbor error: {}",
            self.0, self.1
        )
    }
}

impl std::error::Error for JsonCborDeserializeError {}

/// attempt to deserialize a blob as first json, then cbor
///
/// deserializing a cbor message as json will typically fail very early
pub fn from_json_or_cbor_slice<'de, T: Deserialize<'de>>(
    slice: &'de [u8],
) -> std::result::Result<T, JsonCborDeserializeError> {
    match serde_json::from_slice(slice) {
        Ok(value) => Ok(value),
        Err(json_cause) => match serde_cbor::from_slice(slice) {
            Ok(value) => Ok(value),
            Err(cbor_cause) => Err(JsonCborDeserializeError(json_cause, cbor_cause)),
        },
    }
}

pub fn to_json_or_cbor_vec<T: Serialize>(
    value: T,
    format: SerdeFormat,
) -> std::result::Result<Vec<u8>, JsonCborSerializeError> {
    match format {
        SerdeFormat::Json => {
            let mut blob = serde_json::to_vec(&value).map_err(JsonCborSerializeError::JsonError)?;
            // add newline for the benefit of line based tools for json
            blob.extend_from_slice(b"\n");
            Ok(blob)
        }
        SerdeFormat::Cbor => serde_cbor::to_vec(&value).map_err(JsonCborSerializeError::CborError),
    }
}

/// measure exact CBOR size of an object without actually serializing
pub fn cbor_size<T: Serialize>(value: &T) -> std::result::Result<usize, serde_cbor::error::Error> {
    let mut writer = SizeWrite::new();
    serde_cbor::to_writer(&mut writer, value)?;
    Ok(writer.size)
}

/// measure exact JSON size of an object without actually serializing
pub fn json_size<T: Serialize>(value: &T) -> std::result::Result<usize, serde_json::error::Error> {
    let mut writer = SizeWrite::new();
    serde_json::to_writer(&mut writer, value)?;
    Ok(writer.size)
}

struct SizeWrite {
    size: usize,
}

impl SizeWrite {
    fn new() -> Self {
        SizeWrite { size: 0 }
    }
}

impl Write for &mut SizeWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let res = buf.len();
        self.size += res;
        Ok(res)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serde_json::Value as JsValue;
    use std::num::ParseIntError;

    fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect()
    }

    #[test]
    fn test_cbor_size() {
        assert_eq!(cbor_size(&"test").unwrap(), 5);
        assert_eq!(cbor_size(&"0123456789012345678901234567890123456789").unwrap(), 42);
    }

    #[test]
    fn test_json_size() {
        assert_eq!(json_size(&"test").unwrap(), 6);
        assert_eq!(json_size(&"0123456789012345678901234567890123456789").unwrap(), 42);
    }

    #[test]
    fn test_multi_deserialize() {
        // roundtrip test, happy case
        let test: Vec<JsValue> = vec![json!({}), json!(false), json!("test"), json!(1234)];
        for value in test {
            let cbor = serde_cbor::to_vec(&value).unwrap();
            let json = serde_json::to_vec(&value).unwrap();
            let from_cbor: JsValue = from_json_or_cbor_slice(&cbor).unwrap();
            let from_json: JsValue = from_json_or_cbor_slice(&json).unwrap();
            assert_eq!(from_cbor, value);
            assert_eq!(from_json, value);
        }

        // failure case
        let x: std::result::Result<JsValue, _> = from_json_or_cbor_slice(b"asdasd");
        assert!(x.is_err());

        //interesting case - both valid json and valid cbor
        let x = decode_hex("393939").unwrap();
        let _a: f64 = serde_cbor::from_slice(&x).unwrap();
        let _b: f64 = serde_json::from_slice(&x).unwrap();
    }
}
