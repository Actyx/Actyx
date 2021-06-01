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

use crate::{pair::KeyPair, private::PrivateKey, public::PublicKey, signature::SignedMessage};
use anyhow::{anyhow, bail, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use chacha20poly1305::{
    aead::{AeadInPlace, NewAead},
    XChaCha20Poly1305,
};
use derive_more::{Display, Error};
use parking_lot::RwLock;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::{Into, TryFrom},
    io::{Read, Write},
    sync::Arc,
};

impl From<KeyPair> for libp2p::core::identity::ed25519::Keypair {
    fn from(kp: KeyPair) -> libp2p::core::identity::ed25519::Keypair {
        let mut bytes = kp.to_bytes();
        libp2p::core::identity::ed25519::Keypair::decode(&mut bytes)
            .expect("ed25519 encoding format changed between libp2p and crypto")
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

#[derive(Debug, Display, Error)]
#[display(fmt = "unknown KeyStore version {}", _0)]
pub struct UnknownVersion(#[error(ignore)] u8);

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

    pub fn into_ref(self) -> KeyStoreRef {
        Arc::new(RwLock::new(self))
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

    /// Sign a message with a selection of keys and return it in a SignedMessage envelope
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

    /// Sign a message with the given key, returning only the signature bytes
    ///
    /// ```
    /// let mut store = crypto::KeyStore::default();
    /// let key = store.generate_key_pair().unwrap();
    /// let message = b"hello world";
    /// let signature = store.sign_detached(message, key).unwrap();
    ///
    /// // verify the signature like so:
    /// assert!(key.verify(message, signature.as_ref()));
    /// assert!(!key.verify(&message[1..], signature.as_ref()));
    /// ```
    pub fn sign_detached(&self, message: impl AsRef<[u8]>, key: PublicKey) -> Result<Vec<u8>> {
        let key = self.get_pair(key).ok_or_else(|| anyhow!("key {} not found", key))?;
        let signature = key.sign(message.as_ref());
        Ok(signature.into())
    }

    /// Verify a selection of signatures in the given signed message
    ///
    /// This operation fails if for at least one given key ID:
    ///
    ///  - the key is not known
    ///  - there is no signature with this key on the message
    ///  - the signature is invalid
    ///
    /// See also [`PublicKey::verify`](struct.PublicKey.html#method.verify) for detached signatures.
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
    // can be extracted from Actyx binaries without much hassle, but it does make it a bit
    // less obvious to prying eyes)
    const DUMP_KEY: &'static [u8; 32] = b"uqTmyHA4*G!KQQ@77QMu_xhTg@!o*DnP";
    const VERSION_1: u8 = 1;

    /// Write the state of this store into the given writer
    pub fn dump(&self, mut dst: impl Write) -> Result<()> {
        let mut bytes = serde_cbor::to_vec(self)?;
        let cipher = XChaCha20Poly1305::new(Self::DUMP_KEY.into());
        let mut version_and_nonce = [0u8; 25];
        let (version, nonce) = version_and_nonce.split_at_mut(1);
        // store one byte of version information before the nonce:
        version[0] = Self::VERSION_1;
        // fill the rest with the nonce
        OsRng.fill_bytes(nonce);
        // add the version info as authenticated data
        cipher.encrypt_in_place((&*nonce).into(), version, &mut bytes)?;
        dst.write_all(&version_and_nonce[..])?;
        dst.write_all(&*bytes)?;
        Ok(())
    }

    /// Recreate a store from a reader that yields the bytes previously written by `dump()`
    pub fn restore(mut src: impl Read) -> Result<Self> {
        match src.read_u8()? {
            Self::VERSION_1 => {
                let mut nonce = [0u8; 24];
                src.read_exact(&mut nonce[..])?;
                let cipher = XChaCha20Poly1305::new(Self::DUMP_KEY.into());
                let mut bytes = Vec::new();
                src.read_to_end(&mut bytes)?;
                cipher.decrypt_in_place((&nonce[..]).into(), &[Self::VERSION_1], &mut bytes)?;
                Ok(serde_cbor::from_slice(&*bytes)?)
            }
            v => Err(UnknownVersion(v).into()),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chacha20poly1305::aead;
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

        // check successful case
        let local_restored = KeyStore::restore(&data[..])?;
        assert_eq!(local, local_restored);

        // check corruption
        let n = data.len() - 1;
        let last = data.get_mut(n).unwrap();
        loop {
            let corrupt = rand::random::<u8>();
            if corrupt != *last {
                *last = corrupt;
                break;
            }
        }
        let err = KeyStore::restore(&data[..]).unwrap_err();
        err.downcast_ref::<aead::Error>()
            .unwrap_or_else(|| panic!("found wrong error: {}", err));

        // check unknown version
        data[0] = 0;
        let err = KeyStore::restore(&data[..]).unwrap_err();
        err.downcast_ref::<UnknownVersion>()
            .unwrap_or_else(|| panic!("found wrong error: {}", err));
        assert_eq!(err.to_string(), "unknown KeyStore version 0");

        Ok(())
    }
}
