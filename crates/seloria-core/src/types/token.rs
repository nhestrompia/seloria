use serde::{Deserialize, Serialize};

use crate::crypto::{hash_blake3, Hash, PublicKey};

/// Native token ID (represents the chain's base asset)
pub const NATIVE_TOKEN_ID: Hash = Hash::ZERO;

/// Metadata for a fungible token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMeta {
    /// Unique token ID
    pub token_id: Hash,
    /// Human-readable name
    pub name: String,
    /// Ticker symbol
    pub symbol: String,
    /// Decimal places
    pub decimals: u8,
    /// Total supply (fixed at creation)
    pub total_supply: u64,
    /// Creator/publisher
    pub creator: PublicKey,
}

impl TokenMeta {
    /// Create metadata and compute deterministic token ID
    pub fn new(name: String, symbol: String, decimals: u8, total_supply: u64, creator: PublicKey) -> Self {
        let token_id = compute_token_id(&name, &symbol, decimals, total_supply, &creator);
        TokenMeta {
            token_id,
            name,
            symbol,
            decimals,
            total_supply,
            creator,
        }
    }

    /// Create metadata for the native token
    pub fn native(name: &str, symbol: &str, decimals: u8, total_supply: u64) -> Self {
        TokenMeta {
            token_id: NATIVE_TOKEN_ID,
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            total_supply,
            creator: PublicKey::default(),
        }
    }
}

/// Compute a deterministic token ID from metadata
pub fn compute_token_id(
    name: &str,
    symbol: &str,
    decimals: u8,
    total_supply: u64,
    creator: &PublicKey,
) -> Hash {
    let mut data = Vec::new();
    data.extend_from_slice(creator.as_bytes());
    data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(symbol.as_bytes());
    data.push(decimals);
    data.extend_from_slice(&total_supply.to_le_bytes());
    hash_blake3(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    #[test]
    fn test_token_id_deterministic() {
        let creator = KeyPair::generate();
        let t1 = TokenMeta::new("Token".to_string(), "TOK".to_string(), 6, 1_000_000, creator.public);
        let t2 = TokenMeta::new("Token".to_string(), "TOK".to_string(), 6, 1_000_000, creator.public);
        assert_eq!(t1.token_id, t2.token_id);
    }

    #[test]
    fn test_native_token_id() {
        let meta = TokenMeta::native("Seloria", "SEL", 6, 1_000_000);
        assert_eq!(meta.token_id, NATIVE_TOKEN_ID);
    }
}
