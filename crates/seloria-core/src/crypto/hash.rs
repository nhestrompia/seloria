use serde::{Deserialize, Serialize};
use std::fmt;

/// A 32-byte Blake3 hash
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub const ZERO: Hash = Hash([0u8; 32]);

    pub fn new(data: [u8; 32]) -> Self {
        Hash(data)
    }

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
        Some(Hash(bytes))
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        Self::from_slice(&bytes).ok_or(hex::FromHexError::InvalidStringLength)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_hex())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Compute Blake3 hash of data
pub fn hash_blake3(data: &[u8]) -> Hash {
    let hash = blake3::hash(data);
    Hash(*hash.as_bytes())
}

/// Compute merkle root from a list of hashes
/// Uses Blake3 for internal nodes
pub fn merkle_root(hashes: &[Hash]) -> Hash {
    if hashes.is_empty() {
        return Hash::ZERO;
    }

    if hashes.len() == 1 {
        return hashes[0];
    }

    let mut current_level: Vec<Hash> = hashes.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for chunk in current_level.chunks(2) {
            let combined = if chunk.len() == 2 {
                let mut data = Vec::with_capacity(64);
                data.extend_from_slice(&chunk[0].0);
                data.extend_from_slice(&chunk[1].0);
                hash_blake3(&data)
            } else {
                // Odd number of nodes: duplicate the last one
                let mut data = Vec::with_capacity(64);
                data.extend_from_slice(&chunk[0].0);
                data.extend_from_slice(&chunk[0].0);
                hash_blake3(&data)
            };
            next_level.push(combined);
        }

        current_level = next_level;
    }

    current_level[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_blake3() {
        let data = b"hello world";
        let hash = hash_blake3(data);
        assert_ne!(hash, Hash::ZERO);
    }

    #[test]
    fn test_hash_deterministic() {
        let data = b"test data";
        let hash1 = hash_blake3(data);
        let hash2 = hash_blake3(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = merkle_root(&[]);
        assert_eq!(root, Hash::ZERO);
    }

    #[test]
    fn test_merkle_root_single() {
        let hash = hash_blake3(b"single");
        let root = merkle_root(&[hash]);
        assert_eq!(root, hash);
    }

    #[test]
    fn test_merkle_root_multiple() {
        let hashes: Vec<Hash> = (0..4).map(|i| hash_blake3(&[i])).collect();
        let root = merkle_root(&hashes);
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_hash_hex_roundtrip() {
        let hash = hash_blake3(b"test");
        let hex_str = hash.to_hex();
        let recovered = Hash::from_hex(&hex_str).unwrap();
        assert_eq!(hash, recovered);
    }
}
