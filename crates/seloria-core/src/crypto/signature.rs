use ed25519_dalek::{Signature as DalekSignature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::fmt;

use crate::crypto::keys::{PublicKey, SecretKey};
use crate::error::CoreError;

/// Ed25519 signature (64 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sig(#[serde(with = "BigArray")] pub [u8; 64]);

impl Sig {
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(slice);
        Some(Sig(bytes))
    }

    pub fn from_hex(s: &str) -> Result<Self, CoreError> {
        let bytes = hex::decode(s)?;
        Self::from_slice(&bytes).ok_or(CoreError::InvalidSignature)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Default for Sig {
    fn default() -> Self {
        Sig([0u8; 64])
    }
}

impl fmt::Debug for Sig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sig({}...)", &self.to_hex()[..16])
    }
}

impl fmt::Display for Sig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Sign a message with a secret key
pub fn sign(secret_key: &SecretKey, message: &[u8]) -> Sig {
    let signature = secret_key.signing_key().sign(message);
    Sig(signature.to_bytes())
}

/// Verify a signature against a public key and message
pub fn verify(public_key: &PublicKey, message: &[u8], signature: &Sig) -> Result<(), CoreError> {
    let verifying_key = public_key.to_verifying_key()?;
    let dalek_sig =
        DalekSignature::from_bytes(&signature.0);
    verifying_key
        .verify(message, &dalek_sig)
        .map_err(|_| CoreError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::KeyPair;

    #[test]
    fn test_sign_and_verify() {
        let kp = KeyPair::generate();
        let message = b"hello world";
        let sig = sign(&kp.secret, message);
        assert!(verify(&kp.public, message, &sig).is_ok());
    }

    #[test]
    fn test_verify_wrong_message() {
        let kp = KeyPair::generate();
        let message = b"hello world";
        let sig = sign(&kp.secret, message);
        assert!(verify(&kp.public, b"wrong message", &sig).is_err());
    }

    #[test]
    fn test_verify_wrong_key() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let message = b"hello world";
        let sig = sign(&kp1.secret, message);
        assert!(verify(&kp2.public, message, &sig).is_err());
    }

    #[test]
    fn test_sig_hex_roundtrip() {
        let kp = KeyPair::generate();
        let sig = sign(&kp.secret, b"test");
        let hex_str = sig.to_hex();
        let recovered = Sig::from_hex(&hex_str).unwrap();
        assert_eq!(sig, recovered);
    }
}
