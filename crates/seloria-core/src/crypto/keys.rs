use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::CoreError;

/// Ed25519 public key (32 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

impl PublicKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(PublicKey(bytes))
    }

    pub fn from_hex(s: &str) -> Result<Self, CoreError> {
        let bytes = hex::decode(s)?;
        Self::from_slice(&bytes).ok_or(CoreError::InvalidPublicKey)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Convert to ed25519-dalek VerifyingKey for signature verification
    pub fn to_verifying_key(&self) -> Result<VerifyingKey, CoreError> {
        VerifyingKey::from_bytes(&self.0).map_err(|_| CoreError::InvalidPublicKey)
    }
}

impl Default for PublicKey {
    fn default() -> Self {
        PublicKey([0u8; 32])
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", self.to_hex())
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Ed25519 secret key (32 bytes seed)
/// Not serializable to prevent accidental exposure
#[derive(Clone)]
pub struct SecretKey(SigningKey);

impl SecretKey {
    /// Generate a new random secret key
    pub fn generate() -> Self {
        SecretKey(SigningKey::generate(&mut OsRng))
    }

    /// Create from raw bytes (seed)
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        SecretKey(SigningKey::from_bytes(bytes))
    }

    /// Get the corresponding public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key().to_bytes())
    }

    /// Get the internal signing key for signature operations
    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.0
    }

    /// Export raw bytes (use with caution)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Create from hex string
    pub fn from_hex(s: &str) -> Result<Self, CoreError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(CoreError::InvalidPublicKey);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self::from_bytes(&arr))
    }

    /// Export as hex string (use with caution)
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretKey([REDACTED])")
    }
}

/// A keypair containing both secret and public keys
#[derive(Clone)]
pub struct KeyPair {
    pub secret: SecretKey,
    pub public: PublicKey,
}

impl KeyPair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let secret = SecretKey::generate();
        let public = secret.public_key();
        KeyPair { secret, public }
    }

    /// Create from secret key bytes
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        let secret = SecretKey::from_bytes(bytes);
        let public = secret.public_key();
        KeyPair { secret, public }
    }
}

impl fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPair")
            .field("public", &self.public)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        assert_ne!(kp.public.0, [0u8; 32]);
    }

    #[test]
    fn test_public_key_hex_roundtrip() {
        let kp = KeyPair::generate();
        let hex_str = kp.public.to_hex();
        let recovered = PublicKey::from_hex(&hex_str).unwrap();
        assert_eq!(kp.public, recovered);
    }

    #[test]
    fn test_secret_key_deterministic() {
        let bytes = [42u8; 32];
        let sk1 = SecretKey::from_bytes(&bytes);
        let sk2 = SecretKey::from_bytes(&bytes);
        assert_eq!(sk1.public_key(), sk2.public_key());
    }
}
