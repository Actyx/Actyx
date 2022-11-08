use std::{
    fmt, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    str::FromStr,
};

use crypto::{KeyPair, PrivateKey, PublicKey};
use libp2p::identity;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt};

use crate::cmd::get_data_dir;
use certs::DeveloperCertificate;

const PUB_KEY_FILE_EXTENSION: &str = "pub";
pub const DEFAULT_PRIVATE_KEY_FILE_NAME: &str = "id";

impl fmt::Display for AxPrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pub_hex = self.encode().1;
        write!(f, "{}", pub_hex)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
/// Wrapper around `crypto::PrivateKey` for use inside ax's context. Most notably
/// is the on disk format, which differs from [`crypto::Keystore::dump`].
pub struct AxPrivateKey(PrivateKey);
impl AxPrivateKey {
    fn default_user_identity_dir() -> ActyxOSResult<PathBuf> {
        let p = get_data_dir()?;
        Ok(p.join("keys").join("users"))
    }
    /// Returns the default path for storing user keys
    pub(crate) fn default_user_identity_path() -> ActyxOSResult<PathBuf> {
        let p = Self::default_user_identity_dir()?;
        Ok(p.join(DEFAULT_PRIVATE_KEY_FILE_NAME))
    }
    /// Returns the default for path for storing user keys and creates it, if it
    /// doesn't exist.
    pub fn get_and_create_default_user_identity_dir() -> ActyxOSResult<PathBuf> {
        let p = Self::default_user_identity_dir()?;
        std::fs::create_dir_all(p.clone()).ax_err_ctx(ActyxOSCode::ERR_IO, "Error creating user identity directory")?;
        Ok(p)
    }
    /// Write the private key encoded with a trailing newline into `path`, and the public key into
    /// `path`.pub. Files will be created, if they don't exist. If they do exist already, they will
    /// be truncated. Returns the absolte paths to the private and public key.
    pub fn to_file(&self, path: impl AsRef<Path>) -> ActyxOSResult<(PathBuf, PathBuf)> {
        let priv_path: PathBuf = path.as_ref().into();
        let pub_path: PathBuf = path.as_ref().with_extension(PUB_KEY_FILE_EXTENSION);
        let (priv_hex, pub_hex) = self.encode();
        fs::write(priv_path.clone(), format!("{}\n", priv_hex))
            .ax_err_ctx(ActyxOSCode::ERR_IO, format!("Error writing to {}", priv_path.display()))?;
        fs::write(pub_path.clone(), format!("{}\n", pub_hex))
            .ax_err_ctx(ActyxOSCode::ERR_IO, format!("Error writing to {}", pub_path.display()))?;
        Ok((priv_path, pub_path))
    }

    /// Try to read a private key from a given `path`.
    pub(crate) fn from_file(path: impl AsRef<Path>) -> ActyxOSResult<Self> {
        let path = path.as_ref();
        let mut s = fs::read_to_string(path).map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                ActyxOSError::new(
                    ActyxOSCode::ERR_PATH_INVALID,
                    format!("Path \"{}\" does not exist. Specify an existing path.", path.display()),
                )
            } else {
                ActyxOSError::new(
                    ActyxOSCode::ERR_IO,
                    format!("cannot read file at \"{}\": {}", path.display(), e),
                )
            }
        })?;
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
        }
        Self::decode(s).ax_err_ctx(
            ActyxOSCode::ERR_INVALID_INPUT,
            format!("Error reading from {}", path.display()),
        )
    }

    pub fn to_public(&self) -> PublicKey {
        self.0.into()
    }

    /// Encodes both the private and the associated public key
    fn encode(&self) -> (String, String) {
        let private = format!("{}", self.0);
        let public = format!("{}", self.to_public());
        (private, public)
    }

    fn decode(hex: String) -> ActyxOSResult<Self> {
        let private = PrivateKey::from_str(&hex).ax_invalid_input()?;
        Ok(Self(private))
    }
    /// Generate a new private key
    pub fn generate() -> Self {
        let private = PrivateKey::generate();
        Self(private)
    }
    /// Convert into a key pair to be used with libp2p
    pub(crate) fn to_libp2p_pair(&self) -> identity::Keypair {
        let crypto_kp: KeyPair = self.0.into();
        identity::Keypair::from(crypto_kp)
    }
}
impl FromStr for AxPrivateKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = PrivateKey::from_str(s)?;
        Ok(Self(x))
    }
}

pub fn load_dev_cert(path: Option<PathBuf>) -> ActyxOSResult<DeveloperCertificate> {
    let path = path
        .ok_or(())
        .or_else(|_| ActyxOSResult::Ok(get_data_dir()?.join("certs").join("default")))?;
    let s = fs::read_to_string(path.as_path()).ax_err_ctx(
        ActyxOSCode::ERR_IO,
        format!("failed to read developer certificate at {}", path.display()),
    )?;
    serde_json::from_str(&s).ax_err_ctx(ActyxOSCode::ERR_INVALID_INPUT, "reading developer certificate")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_disk_roundtrip() {
        let temp_folder = tempfile::tempdir().unwrap();

        // Generate a private key
        let private = AxPrivateKey::generate();
        let key_path = temp_folder.path().join("my_key");

        // write both private and public key to distinct files
        private.to_file(&key_path).unwrap();

        // load private key from disk
        let private_from_disk = AxPrivateKey::from_file(&key_path).unwrap();
        assert_eq!(private, private_from_disk);

        // assert written public key
        let public_hex = fs::read_to_string(key_path.with_extension(PUB_KEY_FILE_EXTENSION)).unwrap();
        assert_eq!(public_hex, format!("{}\n", private.encode().1));
    }
}
