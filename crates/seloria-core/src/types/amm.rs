use serde::{Deserialize, Serialize};

use crate::crypto::{hash_blake3, Hash};

/// Basis points denominator
pub const BPS_DENOM: u64 = 10_000;
/// AMM swap fee in basis points (0.1%)
pub const AMM_FEE_BPS: u64 = 10;

/// Metadata for a constant-product AMM pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmPool {
    pub pool_id: Hash,
    pub token_a: Hash,
    pub token_b: Hash,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub lp_supply: u64,
    pub fee_bps: u64,
}

impl AmmPool {
    pub fn new(token_a: Hash, token_b: Hash, reserve_a: u64, reserve_b: u64) -> Self {
        let (a, b, ra, rb) = canonical_pair(token_a, token_b, reserve_a, reserve_b);
        let pool_id = compute_pool_id(a, b);
        AmmPool {
            pool_id,
            token_a: a,
            token_b: b,
            reserve_a: ra,
            reserve_b: rb,
            lp_supply: 0,
            fee_bps: AMM_FEE_BPS,
        }
    }
}

/// Sort token pair and align reserves
pub fn canonical_pair(
    token_a: Hash,
    token_b: Hash,
    reserve_a: u64,
    reserve_b: u64,
) -> (Hash, Hash, u64, u64) {
    if token_a <= token_b {
        (token_a, token_b, reserve_a, reserve_b)
    } else {
        (token_b, token_a, reserve_b, reserve_a)
    }
}

/// Compute deterministic pool ID from token pair
pub fn compute_pool_id(token_a: Hash, token_b: Hash) -> Hash {
    let (a, b) = if token_a <= token_b { (token_a, token_b) } else { (token_b, token_a) };
    let mut data = Vec::new();
    data.extend_from_slice(b"amm");
    data.extend_from_slice(a.as_bytes());
    data.extend_from_slice(b.as_bytes());
    hash_blake3(&data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hash_blake3;

    #[test]
    fn test_pool_id_deterministic() {
        let a = hash_blake3(b"a");
        let b = hash_blake3(b"b");
        let id1 = compute_pool_id(a, b);
        let id2 = compute_pool_id(b, a);
        assert_eq!(id1, id2);
    }
}
