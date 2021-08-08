use crate::public::PublicKey;
use anyhow::{bail, Context, Result};
use byteorder::{BigEndian, ReadBytesExt};
use std::{
    convert::TryFrom,
    io::{Cursor, Read},
};

// FIXME: do we need to change the signature format due to the `PubKey` type?
/// Packed representation of message and signatures:
///
///  - u32 message length ( => LEN )
///  - LEN bytes of message
///  - signatures (1 or more) concatenated
///
/// Each signature starts with a scheme identifier of one octet:
///
///  - 1: Ed25519/SHA512
///      - 32 bytes key ID
///      - 64 bytes signature
///
/// Example:
///
/// ```rust
/// use crypto::SignedMessage;
/// use std::convert::TryFrom;
///
/// # let mut bytes = [0u8; 101];
/// # bytes[4] = 1;
/// let signed = SignedMessage::try_from(&bytes[..])?;
/// # Ok::<(), anyhow::Error>(())
/// ```
#[derive(Debug)]
pub struct SignedMessage(pub(crate) Box<[u8]>);

impl SignedMessage {
    /// Obtain a reference to the message octets that have been signed
    pub fn message(&self) -> &[u8] {
        let mut cursor = Cursor::new(&self.0);
        let msg_len = cursor.read_u32::<BigEndian>().expect("reading from message array") as usize;
        &self.0[4..msg_len + 4]
    }

    /// Extract a vector of key_id–signature pairs attached to this message
    pub fn signatures(&self) -> Vec<(PublicKey, &[u8])> {
        let mut cursor = Cursor::new(&self.0);
        let msg_len = cursor.read_u32::<BigEndian>().expect("reading from message array") as u64;
        cursor.set_position(msg_len + 4);
        let mut res = Vec::new();
        let end = self.0.len() as u64;
        while cursor.position() < end {
            let version = cursor.read_u8().expect("reading from message array");
            assert!(version == 1);
            // TODO: 32?
            let mut key_bytes = [0u8; 32];
            cursor.read_exact(&mut key_bytes).expect("reading from message array");
            let pos = cursor.position() as usize;
            res.push((PublicKey(key_bytes), &self.0[pos..pos + 64]));
            cursor.set_position(cursor.position() + 64);
        }
        res
    }
}

impl AsRef<[u8]> for SignedMessage {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<&[u8]> for SignedMessage {
    type Error = anyhow::Error;
    fn try_from(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);
        let msg_len = cursor.read_u32::<BigEndian>().context("cannot read message length")? as u64;
        let rest = bytes.len() as i64 - 4 - msg_len as i64;
        if rest <= 0 || rest % 97 != 0 {
            bail!("invalid signature length {}", rest);
        }
        cursor.set_position(msg_len + 4);
        let end = bytes.len() as u64;
        while cursor.position() < end {
            let version = cursor.read_u8().expect("reading from message array");
            if version != 1 {
                bail!("invalid signature scheme {}", version);
            }
            cursor.set_position(cursor.position() + 96);
        }
        Ok(Self(bytes.into()))
    }
}
