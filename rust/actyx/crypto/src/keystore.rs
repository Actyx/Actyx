//! A key store that can sign/verify (and later also encrypt/decrypt)
//!
//! The store owns the cryptographic material and ensures proper zeroing before
//! the memory is released.
//!
//! Example use-cases:
//!
//!  - serialize an app ID and the current time, sign it, and base64 it; to be
//!    given to apps such that they can present it as a bearer token to our APIs
//!  - sign the root of an event stream in IPFS
//!  - encrypt Salsa20 stream cipher keys so that they can be stored in IPFS such
//!    that multiple other nodes can decrypt an event stream (given possession of
//!    the private key for which the Salsa20 key was encrypted)

use actyxos_sdk::NodeId;
use aesstream::{AesReader, AesWriter};
use anyhow::{anyhow, bail, Context, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use ed25519::{ExpandedSecretKey, Verifier};
use ed25519_dalek as ed25519;
use fmt::Debug;
use parking_lot::RwLock;
use rand::rngs::OsRng;
use rust_crypto::aessafe::{AesSafe256Decryptor, AesSafe256Encryptor};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::{Into, TryFrom},
    fmt::{self, Display},
    io::{Cursor, Read, Write},
    sync::Arc,
};

/// An ActyxOS private key.
///
/// Currently this is just a newtype wrapper around an ed25519 private key, but this may
/// change if we ever have the need for another encryption standard.
///
/// It seems like SecretKey is often used in the context of symmetric encryption, so we
/// call this PrivateKey, unlike the wrapped type.
#[derive(Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(from = "ed25519::SecretKey", into = "ed25519::SecretKey")]
pub struct PrivateKey([u8; 32]);

impl Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "secret")
    }
}

impl From<ed25519::SecretKey> for PrivateKey {
    fn from(value: ed25519::SecretKey) -> Self {
        PrivateKey(value.to_bytes())
    }
}

impl From<PrivateKey> for ed25519::SecretKey {
    fn from(value: PrivateKey) -> Self {
        Self::from_bytes(&value.0).expect("wrong secret key value for ed25519::SecretKey")
    }
}

impl Into<KeyPair> for PrivateKey {
    fn into(self) -> KeyPair {
        let public: PublicKey = self.into();
        KeyPair { public, private: self }
    }
}
impl Into<PublicKey> for PrivateKey {
    fn into(self) -> PublicKey {
        let secret: ed25519::SecretKey = self.into();
        let public: ed25519::PublicKey = (&secret).into();
        public.into()
    }
}

impl PrivateKey {
    pub fn to_ed25519(self) -> ed25519::SecretKey {
        self.into()
    }
    pub fn to_bytes(&self) -> [u8; ed25519::SECRET_KEY_LENGTH] {
        self.0
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let ed25519 = ed25519::SecretKey::from_bytes(bytes)?;
        Ok(ed25519.into())
    }
    pub fn generate() -> Self {
        let mut csprng = OsRng::default();
        let k = ed25519_dalek::Keypair::generate(&mut csprng);
        k.secret.into()
    }
}

/// Identifier for a public key (and thereby the corresponding private key)
///
/// It consists of 32 octets which are actually the same bytes as the underlying `ed25519::PublicKey`. Thus
/// it's possible to derive all sorts of other identifier from this structure, like a `libp2p::PeerId`.
///
/// A general representation is achieved by base64-encoding the bytes, and prepending an identifier for
/// the key type, which at the moment is only a literal '0' to identify it as an `ed25519::PublicKey`.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct PublicKey([u8; 32]);
impl Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pub_key = self.0;
        let b64 = base64::encode(pub_key);
        write!(f, "0{}", b64)
    }
}
impl Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::str::FromStr for PublicKey {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.to_string();
        if s.is_empty() {
            bail!("empty string");
        }
        let key_type = s.remove(0);
        if key_type != '0' {
            bail!("Unexpected key type {}", key_type);
        }
        let v = base64::decode(s).context("error base64 decoding PubKey")?;
        if v.len() != ed25519::PUBLIC_KEY_LENGTH {
            bail!(
                "Expected {} bytes, received {}",
                ed25519_dalek::PUBLIC_KEY_LENGTH,
                v.len()
            );
        }
        let mut res = [0u8; ed25519::PUBLIC_KEY_LENGTH];
        res.copy_from_slice(&v[..]);
        Ok(Self(res))
    }
}
impl PublicKey {
    /// Gets the underlying ed25519 public key for interop with rust crypto libs
    pub fn to_ed25519(&self) -> ed25519::PublicKey {
        ed25519::PublicKey::from_bytes(&self.0[..]).unwrap()
    }
    pub fn to_bytes(&self) -> [u8; ed25519::PUBLIC_KEY_LENGTH] {
        let mut bytes = [0u8; ed25519::PUBLIC_KEY_LENGTH];
        bytes[..].copy_from_slice(&self.0[..]);
        bytes
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let ed25519 = ed25519::PublicKey::from_bytes(bytes)?;
        Ok(ed25519.into())
    }
    fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let signature = if let Ok(sig) = ed25519::Signature::try_from(signature) {
            sig
        } else {
            return false;
        };
        self.to_ed25519().verify(message, &signature).is_ok()
    }
}
impl Into<libp2p::core::PeerId> for PublicKey {
    fn into(self) -> libp2p::core::PeerId {
        let public = self.into();
        libp2p::core::PeerId::from_public_key(public)
    }
}
impl TryFrom<libp2p::core::PeerId> for PublicKey {
    type Error = anyhow::Error;
    // This only works if the multi_hash with this peer_id is encoded is the identity hash,
    // and the underlying public key is ed25519
    fn try_from(peer_id: libp2p::core::PeerId) -> Result<Self, Self::Error> {
        match multihash::Multihash::from_bytes(&peer_id.to_bytes()) {
            Ok(multihash) => {
                if multihash.code() == u64::from(multihash::Code::Identity) {
                    let bytes = multihash.digest();
                    let libp2p_pubkey = libp2p::core::identity::PublicKey::from_protobuf_encoding(bytes)?;
                    match libp2p_pubkey {
                        libp2p::core::identity::PublicKey::Ed25519(ed25519_pub) => {
                            let bytes = ed25519_pub.encode();

                            let pub_key = ed25519::PublicKey::from_bytes(&bytes[..])
                                .map_err(|e| anyhow!(e))
                                .context("Not a valid ed25519_dalek::PublicKey")?;
                            Ok(pub_key.into())
                        }
                        _ => bail!("Expected ed25519::PublicKey!"),
                    }
                } else {
                    bail!("Only PeerIds encoded with identity hash can be decoded")
                }
            }

            Err(err) => bail!(err),
        }
    }
}

impl Into<libp2p::core::identity::PublicKey> for PublicKey {
    fn into(self) -> libp2p::core::identity::PublicKey {
        libp2p::core::identity::PublicKey::Ed25519(
            libp2p::core::identity::ed25519::PublicKey::decode(&self.0)
                .expect("ed25519 encoding format changed between libp2p and crypto"),
        )
    }
}
impl From<libp2p::core::identity::ed25519::PublicKey> for PublicKey {
    fn from(o: libp2p::core::identity::ed25519::PublicKey) -> Self {
        Self(o.encode())
    }
}

impl Into<libp2p::core::identity::ed25519::Keypair> for KeyPair {
    fn into(self) -> libp2p::core::identity::ed25519::Keypair {
        let mut bytes = self.to_bytes();
        libp2p::core::identity::ed25519::Keypair::decode(&mut bytes)
            .expect("ed25519 encoding format changed between libp2p and crypto")
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<ed25519::PublicKey> for PublicKey {
    fn from(key: ed25519::PublicKey) -> Self {
        Self(*key.as_bytes())
    }
}

impl From<NodeId> for PublicKey {
    fn from(node_id: NodeId) -> Self {
        let mut res = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
        res.copy_from_slice(node_id.as_ref());
        Self(res)
    }
}
impl Into<NodeId> for PublicKey {
    fn into(self) -> NodeId {
        NodeId::from_bytes(self.as_ref()).unwrap()
    }
}

/// A keypair.
///
/// Conceptually, this is a generic keypair. But currently we only support ed25519 encryption.
#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct KeyPair {
    public: PublicKey,
    private: PrivateKey,
}

impl PartialEq for KeyPair {
    fn eq(&self, other: &Self) -> bool {
        self.public == other.public
    }
}
impl Eq for KeyPair {}

impl Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPair").field("public", &self.public).finish()
    }
}

impl KeyPair {
    pub fn pub_key(&self) -> PublicKey {
        self.public
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let secret_key = self.private.to_ed25519();
        ExpandedSecretKey::from(&secret_key)
            .sign(message, &self.public.to_ed25519())
            .to_bytes()
    }

    /// Convert this keypair to bytes.
    ///
    /// # Returns
    //
    /// An array of bytes, `[u8; KEYPAIR_LENGTH]`.  The first
    /// `SECRET_KEY_LENGTH` of bytes is the `SecretKey`, and the next
    /// `PUBLIC_KEY_LENGTH` bytes is the `PublicKey` (the same as other
    /// libraries, such as [Adam Langley's ed25519 Golang
    /// implementation](https://github.com/agl/ed25519/)).
    ///
    /// Copied from ed2559_dalek::KeyPair::to_bytes
    pub fn to_bytes(&self) -> [u8; ed25519_dalek::PUBLIC_KEY_LENGTH + ed25519_dalek::SECRET_KEY_LENGTH] {
        let mut bytes: [u8; 64] = [0u8; 64];

        bytes[..ed25519_dalek::SECRET_KEY_LENGTH].copy_from_slice(&self.private.0);
        bytes[ed25519_dalek::SECRET_KEY_LENGTH..].copy_from_slice(&self.public.0);
        bytes
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = PublicKey;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("PublicKey")
            }
            fn visit_str<E: serde::de::Error>(self, string: &str) -> Result<Self::Value, E> {
                use std::str::FromStr;
                PublicKey::from_str(string).map_err(serde::de::Error::custom)
            }
        }
        deserializer.deserialize_str(V)
    }
}

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
pub struct SignedMessage(Box<[u8]>);

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

pub type KeyStoreRef = Arc<RwLock<KeyStore>>;

type DumpFn = Box<dyn Fn(Box<[u8]>) -> Result<()> + Send + Sync>;

/// Central entry point for crypto operations.
///
/// The KeyStore holds a number of keys, either complete pairs or only public keys.
/// These keys are referenced by PublicKey.
#[derive(Serialize, Deserialize)]
pub struct KeyStore {
    pairs: BTreeMap<PublicKey, PrivateKey>,
    publics: BTreeSet<PublicKey>,
    #[serde(skip)]
    dump_after_modify: Option<DumpFn>,
}
impl std::cmp::PartialEq for KeyStore {
    fn eq(&self, other: &Self) -> bool {
        self.pairs == other.pairs && self.publics == other.publics
    }
}
impl std::cmp::Eq for KeyStore {}
impl std::fmt::Debug for KeyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyStore")
            .field("pairs", &self.pairs)
            .field("publics", &self.publics)
            .finish()
    }
}

impl Default for KeyStore {
    fn default() -> Self {
        Self {
            pairs: BTreeMap::new(),
            publics: BTreeSet::new(),
            dump_after_modify: None,
        }
    }
}

impl KeyStore {
    /// Installs a callback, which is called after every mutation to the held keys
    ///
    /// This is equivalent to calling KeyStore::dump() after the mutating function
    pub fn with_cb(mut self, dump_callback: DumpFn) -> Self {
        self.dump_after_modify = Some(dump_callback);
        self
    }

    fn dump_if_cb_installed(&mut self) -> Result<()> {
        if let Some(fun) = &self.dump_after_modify {
            let mut vec = vec![];
            self.dump(&mut vec)?;
            fun(vec.into())
        } else {
            Ok(())
        }
    }

    /// Generate a new Ed25519 key pair and return its key_id
    ///
    /// The key pair is stored in this KeyStore. Callers should make sure to persist the
    /// store with `dump()`.
    pub fn generate_key_pair(&mut self) -> Result<PublicKey> {
        let private = PrivateKey::generate();
        let key: PublicKey = private.into();
        self.pairs.insert(key, private);
        self.dump_if_cb_installed()?;
        Ok(key)
    }

    /// Add an Ed25519 key pair to this store
    pub fn add_key_pair_ed25519(&mut self, pair: ed25519_dalek::Keypair) -> Result<PublicKey> {
        let key = pair.public.into();
        self.pairs.entry(key).or_insert_with(|| pair.secret.into());
        self.dump_if_cb_installed()?;
        Ok(key)
    }

    /// Add an Ed25519 public key to this store
    pub fn add_public_key_ed25519(&mut self, key: ed25519_dalek::PublicKey) -> Result<PublicKey> {
        let key = key.into();
        self.publics.insert(key);
        self.dump_if_cb_installed()?;
        Ok(key)
    }

    /// Sign a message with a selection of keys
    ///
    /// The message must not be larger than u32::MAX bytes and at least one key_id must
    /// be given.
    pub fn sign(&self, message: impl AsRef<[u8]>, keys: impl IntoIterator<Item = PublicKey>) -> Result<SignedMessage> {
        let keys = keys.into_iter();
        let size = keys.size_hint().0 * 97 + message.as_ref().len() + 4;
        let mut out = Vec::with_capacity(size);
        if let Ok(len) = u32::try_from(message.as_ref().len()) {
            out.write_u32::<BigEndian>(len).expect("writing to message buffer");
        } else {
            bail!("message is too long: {} > {}", message.as_ref().len(), u32::MAX);
        }
        out.write_all(message.as_ref()).expect("writing to message buffer");
        let mut signed = false;
        for key_id in keys {
            signed = true;
            if let Some(key) = self.get_pair(key_id) {
                let signature = key.sign(message.as_ref());
                out.write_u8(1).expect("writing to message buffer");
                out.write_all(key_id.as_ref()).expect("writing to message buffer");
                out.write_all(&signature).expect("writing to message buffer");
            } else {
                bail!("key not found: {}", key_id);
            }
        }
        if !signed {
            bail!("no keys selected");
        }
        Ok(SignedMessage(out.into()))
    }

    /// Verify a selection of signatures in the given signed message
    ///
    /// This operation fails if for at least one given key ID:
    ///
    ///  - the key is not known
    ///  - there is no signature with this key on the message
    ///  - the signature is invalid
    pub fn verify(&self, message: &SignedMessage, keys: impl IntoIterator<Item = PublicKey>) -> Result<()> {
        let msg = message.message();
        let sigs = message.signatures().into_iter().collect::<BTreeMap<_, _>>();
        for key in keys {
            if let Some(sig) = sigs.get(&key) {
                if !key.verify(msg, *sig) {
                    bail!("invalid signature for {}", key);
                }
            } else {
                bail!("required signature not found for {}", key);
            }
        }
        Ok(())
    }

    pub fn get_pair(&self, public: PublicKey) -> Option<KeyPair> {
        self.pairs.get(&public).map(|private| KeyPair {
            public,
            private: *private,
        })
    }

    pub fn is_pair_available(&self, key_id: &PublicKey) -> bool {
        self.pairs.get(key_id).is_some()
    }

    pub fn get_pairs(&self) -> &BTreeMap<PublicKey, PrivateKey> {
        &self.pairs
    }

    pub fn get_pub_keys(&self) -> BTreeSet<PublicKey> {
        self.pairs.keys().chain(self.publics.iter()).copied().collect()
    }

    // dumps are obfuscated with this key (this does not provide much security since the key
    // can be extracted from ActyxOS binaries without much hassle, but it does make it a bit
    // less obvious to prying eyes)
    const DUMP_KEY: &'static [u8] = b"uqTmyHA4*G!KQQ@77QMu_xhTg@!o*DnP";

    /// Write the state of this store into the given writer
    pub fn dump(&self, dst: impl Write) -> Result<()> {
        let enc = AesSafe256Encryptor::new(Self::DUMP_KEY);
        let mut writer = AesWriter::new(dst, enc)?;
        serde_cbor::to_writer(&mut writer, self)?;
        writer.flush()?;
        Ok(())
    }

    /// Recreate a store from a reader that yields the bytes previously written by `dump()`
    pub fn restore(src: impl Read) -> Result<Self> {
        let dec = AesSafe256Decryptor::new(Self::DUMP_KEY);
        let reader = AesReader::new(src, dec)?;
        Ok(serde_cbor::from_reader(reader)?)
    }

    /// recreate the keystore from v0 format (ActyxOS 1.0)
    pub fn restore_v0(src: impl Read) -> Result<Self> {
        let dec = AesSafe256Decryptor::new(Self::DUMP_KEY);
        let reader = AesReader::new(src, dec)?;
        let keystore_v0: v0::KeyStore = serde_cbor::from_reader(reader)?;
        Ok(keystore_v0.into())
    }

    /// Restores a KeyStore from a given file; starts out empty if the file doesn't exist.
    pub fn restore_or_empty<P: AsRef<std::path::Path>>(src: P) -> Result<Self> {
        use std::io::ErrorKind;
        match std::fs::File::open(src) {
            Ok(fd) => Self::restore(fd),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err.into()),
        }
    }
}

/// module for read compatibility with actyxos v1.0 keystores
///
/// this is mostly copied and stripped down from the actyxos 1.0 codebase.
mod v0 {
    use super::*;
    use serde::de;

    #[derive(Deserialize, Serialize)]
    pub struct KeyPair {
        pub public: ed25519::PublicKey,
        pub private: PrivateKey, // our PrivateKey serializes the same as ed25519::SecretKey
    }

    impl From<KeyPair> for super::KeyPair {
        fn from(value: KeyPair) -> Self {
            Self {
                public: value.public.into(),
                private: value.private,
            }
        }
    }

    fn deserialize32<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<[u8; 32], D::Error> {
        struct X;
        impl<'de> Visitor<'de> for X {
            type Value = [u8; 32];
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "32 bytes")
            }
            fn visit_bytes<E: de::Error>(self, mut v: &[u8]) -> std::result::Result<Self::Value, E> {
                if v.len() != 32 {
                    return Err(de::Error::custom(format!("found {} bytes", v.len())));
                }
                let mut res = [0u8; 32];
                v.read_exact(&mut res).expect("reading from slice");
                Ok(res)
            }
        }
        d.deserialize_bytes(X)
    }

    #[derive(Ord, PartialOrd, Eq, PartialEq, Deserialize)]
    pub struct KeyId(#[serde(deserialize_with = "deserialize32")] [u8; 32]);

    #[derive(Deserialize)]
    pub struct KeyStore {
        pub pairs: BTreeMap<KeyId, KeyPair>,
        pub publics: BTreeMap<KeyId, ed25519::PublicKey>,
    }

    impl From<KeyStore> for super::KeyStore {
        fn from(io: KeyStore) -> Self {
            // forget the key ids and replace them with the keys
            let pairs = io
                .pairs
                .into_iter()
                .map(|(_, v)| (v.public.into(), v.private))
                .collect::<BTreeMap<PublicKey, PrivateKey>>();
            let publics = io
                .publics
                .into_iter()
                .map(|(_, v)| v.into())
                .collect::<BTreeSet<PublicKey>>();
            Self {
                pairs,
                publics,
                dump_after_modify: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn must_sign_and_verify() {
        let mut store = KeyStore::default();
        let me = store.generate_key_pair().unwrap();
        let other = store.generate_key_pair().unwrap();
        let third = store.generate_key_pair().unwrap();

        let message = b"hello world!";
        let signed = store.sign(message, vec![me, other]).unwrap();

        assert_eq!(signed.as_ref().len(), message.len() + 4 + 2 * (1 + 32 + 64));
        assert_eq!(signed.message(), message);

        store.verify(&signed, vec![me, other]).unwrap();

        assert!(store
            .verify(&signed, vec![other, third])
            .unwrap_err()
            .to_string()
            .starts_with("required signature not found"));

        let mut broken = Vec::from(signed.as_ref());
        broken[8] = b'H';
        let broken_sig = SignedMessage::try_from(&*broken).unwrap();
        assert!(store
            .verify(&broken_sig, vec![me])
            .unwrap_err()
            .to_string()
            .starts_with("invalid signature"));
    }

    #[test]
    fn must_read_signed_message() {
        let mut valid = [0u8; 101];
        valid[4] = 1;
        let msg = SignedMessage::try_from(&valid[..]).unwrap();
        assert_eq!(msg.message(), &valid[..0]);
        assert_eq!(msg.signatures(), vec![(PublicKey([0u8; 32]), &valid[5..69])]);

        let invalid = [0u8; 9];
        assert!(SignedMessage::try_from(&invalid[..])
            .unwrap_err()
            .to_string()
            .starts_with("invalid signature length"));
    }

    #[test]
    fn must_dump_and_restore() {
        let mut store = KeyStore::default();
        let me = store.generate_key_pair().unwrap();
        let message = b"hello world?";
        let signed = store.sign(message, vec![me]).unwrap();

        let mut bytes = Vec::new();
        store.dump(&mut bytes).unwrap();

        let store2 = KeyStore::restore(&*bytes).unwrap();
        store2.verify(&signed, vec![me]).unwrap();
        assert_eq!(store2, store);
    }

    #[test]
    fn must_create_an_empty_keystore() {
        KeyStore::restore_or_empty("/tmp/doesntexist").unwrap();
    }

    #[test]
    fn pub_key_string_roundtrip() {
        let mut store = KeyStore::default();
        let public = store.generate_key_pair().unwrap();
        let str = public.to_string();
        let public_0 = str.parse().unwrap();
        assert_eq!(public, public_0);

        let public_1: PublicKey = serde_cbor::from_slice(&serde_cbor::to_vec(&public).unwrap()[..]).unwrap();
        assert_eq!(public, public_1);
    }

    #[test]
    fn peer_id_pub_key_roundtrip() {
        let mut store = KeyStore::default();
        let public = store.generate_key_pair().unwrap();

        let peer_id: libp2p::core::PeerId = public.clone().into();

        let public_from_peer_id: PublicKey = peer_id.try_into().unwrap();
        assert_eq!(public, public_from_peer_id);
    }

    #[test]
    fn must_read_v0_keystore() -> anyhow::Result<()> {
        // base64 encoded v0 keystore
        let base64 = "vyd2rbz4I2taT1dkZAW+E/ZtsdJ9nnjHvwooT6x1sXzrYho2C6UpkYAM3wENLth4mFqi3Ncqq06VDNOnmwtf+dwWoh9RP2Ozwp/+d56BHPyT14/3edfgPHWv4OYag8vixyvEqWdBU5T4u/y8m7yFjQ1GEci8Qep+DkRwRJcg4IwyjmVdESRTOjVeue1sVV7dzV0H7IPhWAt4g0fn1Tya4A==";
        let bytes = base64::decode(base64)?;
        let _ = KeyStore::restore_v0(&bytes[..])?;
        Ok(())
    }

    #[test]
    fn must_read_v1_keystore() -> anyhow::Result<()> {
        // base64 encoded v0 keystore
        let base64 = "V9DuJKgD3E7GEypiWNdV2Ugx6e6W2E87BYeWkvPTXhczxIwRL3dcbHlYYTBq/j5zP0rD7IdSpCuKQqOiJ09aYTxwLfOpf/zhjEWeQkvJJqJxe8LY8vLq++RASTNu1pB2WLM0Xro7Il/TNpizH0gMcbzZFyTbye2NWOXiejbBPAU=";
        let bytes = base64::decode(base64)?;
        let _ = KeyStore::restore(&bytes[..])?;
        Ok(())
    }

    #[test]
    fn keystore_roundtrip() -> anyhow::Result<()> {
        // generate keystore filled with random keys
        let mut local = KeyStore::default();
        let mut remote = KeyStore::default();
        for _ in 0..10 {
            local.generate_key_pair()?;
            local.add_public_key_ed25519(remote.generate_key_pair()?.to_ed25519())?;
        }
        // encrypt/serialize and deserialize/decrypt
        let mut data = Vec::new();
        local.dump(&mut data)?;
        let local_restored = KeyStore::restore(&data[..])?;
        assert_eq!(local, local_restored);
        Ok(())
    }
}
