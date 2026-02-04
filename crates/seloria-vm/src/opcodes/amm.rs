use seloria_core::{
    canonical_pair, compute_pool_id, AmmPool, BPS_DENOM, Hash, PublicKey,
};
use seloria_state::{ChainState, Storage};
use tracing::debug;

use crate::error::VmError;

/// Execute POOL_CREATE operation
pub fn execute_pool_create<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    token_a: &Hash,
    token_b: &Hash,
    amount_a: u64,
    amount_b: u64,
) -> Result<Hash, VmError> {
    if token_a == token_b {
        return Err(VmError::InvalidOperation("Pool tokens must differ".to_string()));
    }
    if amount_a == 0 || amount_b == 0 {
        return Err(VmError::InvalidOperation(
            "Pool liquidity amounts must be > 0".to_string(),
        ));
    }
    if state.get_token(token_a).is_none() && *token_a != seloria_core::NATIVE_TOKEN_ID {
        return Err(VmError::TokenNotFound(token_a.to_hex()));
    }
    if state.get_token(token_b).is_none() && *token_b != seloria_core::NATIVE_TOKEN_ID {
        return Err(VmError::TokenNotFound(token_b.to_hex()));
    }

    let (a, b, ra, rb) = canonical_pair(*token_a, *token_b, amount_a, amount_b);
    let pool_id = compute_pool_id(a, b);
    if state.get_pool(&pool_id).is_some() {
        return Err(VmError::PoolExists(pool_id.to_hex()));
    }

    let balance_a = state.get_token_balance(sender, &a);
    if balance_a < ra {
        return Err(VmError::InsufficientBalance {
            have: balance_a,
            need: ra,
        });
    }
    let balance_b = state.get_token_balance(sender, &b);
    if balance_b < rb {
        return Err(VmError::InsufficientBalance {
            have: balance_b,
            need: rb,
        });
    }

    // Debit sender and create pool
    state.debit_token(sender, &a, ra)?;
    state.debit_token(sender, &b, rb)?;

    let mut pool = AmmPool::new(a, b, ra, rb);
    let lp_minted = integer_sqrt((ra as u128) * (rb as u128));
    if lp_minted == 0 {
        return Err(VmError::InvalidOperation("LP mint amount is zero".to_string()));
    }
    pool.lp_supply = lp_minted;

    state.add_pool(pool.clone());
    state.credit_lp(&pool.pool_id, sender, lp_minted);

    debug!(
        "Created pool {} with reserves {} / {} and LP {}",
        pool.pool_id, ra, rb, lp_minted
    );

    Ok(pool.pool_id)
}

/// Execute POOL_ADD operation
pub fn execute_pool_add<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    pool_id: &Hash,
    amount_a: u64,
    amount_b: u64,
    min_lp: u64,
) -> Result<u64, VmError> {
    if amount_a == 0 || amount_b == 0 {
        return Err(VmError::InvalidOperation(
            "Pool add amounts must be > 0".to_string(),
        ));
    }

    let pool = state
        .get_pool(pool_id)
        .cloned()
        .ok_or_else(|| VmError::PoolNotFound(pool_id.to_hex()))?;
    if pool.lp_supply == 0 || pool.reserve_a == 0 || pool.reserve_b == 0 {
        return Err(VmError::InvalidOperation("Pool has zero liquidity".to_string()));
    }

    let balance_a = state.get_token_balance(sender, &pool.token_a);
    if balance_a < amount_a {
        return Err(VmError::InsufficientBalance {
            have: balance_a,
            need: amount_a,
        });
    }
    let balance_b = state.get_token_balance(sender, &pool.token_b);
    if balance_b < amount_b {
        return Err(VmError::InsufficientBalance {
            have: balance_b,
            need: amount_b,
        });
    }

    let lp_from_a = (amount_a as u128) * (pool.lp_supply as u128) / (pool.reserve_a as u128);
    let lp_from_b = (amount_b as u128) * (pool.lp_supply as u128) / (pool.reserve_b as u128);
    let lp_minted = lp_from_a.min(lp_from_b) as u64;
    if lp_minted == 0 {
        return Err(VmError::InvalidOperation("LP mint amount is zero".to_string()));
    }
    if lp_minted < min_lp {
        return Err(VmError::SlippageExceeded);
    }

    state.debit_token(sender, &pool.token_a, amount_a)?;
    state.debit_token(sender, &pool.token_b, amount_b)?;

    let pool_mut = state.get_pool_mut(pool_id).unwrap();
    pool_mut.reserve_a = pool_mut.reserve_a.saturating_add(amount_a);
    pool_mut.reserve_b = pool_mut.reserve_b.saturating_add(amount_b);
    pool_mut.lp_supply = pool_mut.lp_supply.saturating_add(lp_minted);

    state.credit_lp(pool_id, sender, lp_minted);

    debug!(
        "Added liquidity to pool {}: {} / {} minted LP {}",
        pool_id, amount_a, amount_b, lp_minted
    );

    Ok(lp_minted)
}

/// Execute POOL_REMOVE operation
pub fn execute_pool_remove<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    pool_id: &Hash,
    lp_amount: u64,
    min_a: u64,
    min_b: u64,
) -> Result<(u64, u64), VmError> {
    if lp_amount == 0 {
        return Err(VmError::InvalidOperation("LP amount must be > 0".to_string()));
    }

    let pool = state
        .get_pool(pool_id)
        .cloned()
        .ok_or_else(|| VmError::PoolNotFound(pool_id.to_hex()))?;
    if pool.lp_supply == 0 || pool.reserve_a == 0 || pool.reserve_b == 0 {
        return Err(VmError::InvalidOperation("Pool has zero liquidity".to_string()));
    }

    let lp_balance = state.get_lp_balance(pool_id, sender);
    if lp_balance < lp_amount {
        return Err(VmError::InsufficientBalance {
            have: lp_balance,
            need: lp_amount,
        });
    }

    let amount_a = (lp_amount as u128) * (pool.reserve_a as u128) / (pool.lp_supply as u128);
    let amount_b = (lp_amount as u128) * (pool.reserve_b as u128) / (pool.lp_supply as u128);
    let amount_a = amount_a as u64;
    let amount_b = amount_b as u64;

    if amount_a < min_a || amount_b < min_b {
        return Err(VmError::SlippageExceeded);
    }

    state.debit_lp(pool_id, sender, lp_amount)?;

    let pool_mut = state.get_pool_mut(pool_id).unwrap();
    pool_mut.reserve_a = pool_mut.reserve_a.saturating_sub(amount_a);
    pool_mut.reserve_b = pool_mut.reserve_b.saturating_sub(amount_b);
    pool_mut.lp_supply = pool_mut.lp_supply.saturating_sub(lp_amount);

    state.credit_token(sender, &pool.token_a, amount_a);
    state.credit_token(sender, &pool.token_b, amount_b);

    debug!(
        "Removed liquidity from pool {}: {} / {} burned LP {}",
        pool_id, amount_a, amount_b, lp_amount
    );

    Ok((amount_a, amount_b))
}

/// Execute SWAP operation
pub fn execute_swap<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    pool_id: &Hash,
    token_in: &Hash,
    amount_in: u64,
    min_out: u64,
) -> Result<u64, VmError> {
    if amount_in == 0 {
        return Err(VmError::InvalidOperation("Swap amount must be > 0".to_string()));
    }

    let pool = state
        .get_pool(pool_id)
        .cloned()
        .ok_or_else(|| VmError::PoolNotFound(pool_id.to_hex()))?;

    let (reserve_in, reserve_out, token_out) = if *token_in == pool.token_a {
        (pool.reserve_a, pool.reserve_b, pool.token_b)
    } else if *token_in == pool.token_b {
        (pool.reserve_b, pool.reserve_a, pool.token_a)
    } else {
        return Err(VmError::InvalidOperation("Token not in pool".to_string()));
    };
    if reserve_in == 0 || reserve_out == 0 {
        return Err(VmError::InvalidOperation("Pool has zero liquidity".to_string()));
    }

    let sender_balance = state.get_token_balance(sender, token_in);
    if sender_balance < amount_in {
        return Err(VmError::InsufficientBalance {
            have: sender_balance,
            need: amount_in,
        });
    }

    let amount_in_with_fee = (amount_in as u128) * (BPS_DENOM as u128 - pool.fee_bps as u128)
        / (BPS_DENOM as u128);
    let numerator = amount_in_with_fee * (reserve_out as u128);
    let denominator = (reserve_in as u128) + amount_in_with_fee;
    let amount_out = (numerator / denominator) as u64;

    if amount_out == 0 || amount_out < min_out {
        return Err(VmError::SlippageExceeded);
    }

    state.debit_token(sender, token_in, amount_in)?;
    state.credit_token(sender, &token_out, amount_out);

    let pool_mut = state.get_pool_mut(pool_id).unwrap();
    if *token_in == pool_mut.token_a {
        pool_mut.reserve_a = pool_mut.reserve_a.saturating_add(amount_in);
        pool_mut.reserve_b = pool_mut.reserve_b.saturating_sub(amount_out);
    } else {
        pool_mut.reserve_b = pool_mut.reserve_b.saturating_add(amount_in);
        pool_mut.reserve_a = pool_mut.reserve_a.saturating_sub(amount_out);
    }

    debug!(
        "Swap in pool {}: in {} out {}",
        pool_id, amount_in, amount_out
    );

    Ok(amount_out)
}

fn integer_sqrt(n: u128) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x0 = n;
    let mut x1 = (x0 + 1) >> 1;
    while x1 < x0 {
        x0 = x1;
        x1 = (x1 + n / x1) >> 1;
    }
    x0 as u64
}
