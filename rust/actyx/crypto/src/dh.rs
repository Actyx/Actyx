use curve25519_dalek::edwards::CompressedEdwardsY;
use sha2::{Digest, Sha512};
use zeroize::Zeroize;

/// Construct a X25519 secret key from a Ed25519 secret key.
///
/// > **Note**: If the Ed25519 secret key is already used in the context
/// > of other cryptographic protocols outside of Noise, e.g. for
/// > signing in the `secio` protocol, it should be preferred to
/// > create a new keypair for use in the Noise protocol.
/// >
/// > See also:
/// >
/// >  * [Noise: Static Key Reuse](http://www.noiseprotocol.org/noise.html#security-considerations)
/// >  * [Ed25519 to Curve25519](https://libsodium.gitbook.io/doc/advanced/ed25519-curve25519)
pub fn ed25519_to_x25519_sk(ed25519_sk: &ed25519_dalek::SecretKey) -> x25519_dalek::StaticSecret {
    // An Ed25519 public key is derived off the left half of the SHA512 of the
    // secret scalar, hence a matching conversion of the secret key must do
    // the same to yield a Curve25519 keypair with the same public key.
    let mut curve25519_sk: [u8; 32] = [0; 32];
    let hash = Sha512::digest(ed25519_sk.as_ref());
    curve25519_sk.copy_from_slice(&hash.as_slice()[..32]);
    let sk = x25519_dalek::StaticSecret::from(curve25519_sk); // Copy
    curve25519_sk.zeroize();
    sk
}

/// Construct a curve25519 public key from an Ed25519 public key.
pub fn ed25519_to_x25519_pk(pk: &ed25519_dalek::PublicKey) -> x25519_dalek::PublicKey {
    x25519_dalek::PublicKey::from(
        CompressedEdwardsY(pk.to_bytes())
            .decompress()
            .expect("An Ed25519 public key is a valid point by construction.")
            .to_montgomery()
            .0,
    )
}
