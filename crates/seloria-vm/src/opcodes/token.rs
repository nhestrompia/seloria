use seloria_core::{TokenMeta, NATIVE_TOKEN_ID, PublicKey};
use seloria_state::{ChainState, Storage};
use tracing::debug;

use crate::error::VmError;

/// Execute TOKEN_CREATE operation
pub fn execute_token_create<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    name: &str,
    symbol: &str,
    decimals: u8,
    total_supply: u64,
) -> Result<TokenMeta, VmError> {
    if name.trim().is_empty() || symbol.trim().is_empty() {
        return Err(VmError::InvalidOperation("Token name/symbol required".to_string()));
    }
    if total_supply == 0 {
        return Err(VmError::InvalidOperation("Token supply must be > 0".to_string()));
    }

    let meta = TokenMeta::new(
        name.to_string(),
        symbol.to_string(),
        decimals,
        total_supply,
        *sender,
    );

    if meta.token_id == NATIVE_TOKEN_ID {
        return Err(VmError::InvalidOperation(
            "Token ID conflicts with native token".to_string(),
        ));
    }

    if state.get_token(&meta.token_id).is_some() {
        return Err(VmError::TokenExists(meta.token_id.to_hex()));
    }

    state.add_token(meta.clone());
    state.credit_token(sender, &meta.token_id, total_supply);

    debug!(
        "Created token {} ({}) with supply {}",
        meta.token_id, meta.symbol, total_supply
    );

    Ok(meta)
}

/// Execute TOKEN_TRANSFER operation
pub fn execute_token_transfer<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    token_id: &seloria_core::Hash,
    to: &PublicKey,
    amount: u64,
) -> Result<(), VmError> {
    if amount == 0 {
        return Err(VmError::InvalidOperation("Transfer amount must be > 0".to_string()));
    }
    if state.get_token(token_id).is_none() && *token_id != NATIVE_TOKEN_ID {
        return Err(VmError::TokenNotFound(token_id.to_hex()));
    }

    let sender_balance = state.get_token_balance(sender, token_id);
    if sender_balance < amount {
        return Err(VmError::InsufficientBalance {
            have: sender_balance,
            need: amount,
        });
    }

    state.transfer_token(sender, to, token_id, amount)?;

    debug!("Transferred {} of {} from {} to {}", amount, token_id, sender, to);

    Ok(())
}
