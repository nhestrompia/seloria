use std::collections::BTreeMap;

use seloria_core::{Capability, Hash, Op, Transaction, NATIVE_TOKEN_ID};
use seloria_state::{ChainState, Storage};

use crate::error::VmError;

/// Transaction validation result
pub struct ValidationResult {
    pub is_valid: bool,
    pub error: Option<VmError>,
    pub required_balance: u64,
}

impl ValidationResult {
    pub fn ok(required_balance: u64) -> Self {
        ValidationResult {
            is_valid: true,
            error: None,
            required_balance,
        }
    }

    pub fn err(error: VmError) -> Self {
        ValidationResult {
            is_valid: false,
            error: Some(error),
            required_balance: 0,
        }
    }
}

/// Validate a transaction before execution
pub fn validate_transaction<S: Storage>(
    tx: &Transaction,
    state: &ChainState<S>,
    current_time: u64,
) -> ValidationResult {
    // 1. Verify signature
    if let Err(_) = tx.verify_signature() {
        return ValidationResult::err(VmError::InvalidSignature);
    }

    // 2. Check sender is certified agent (unless they're registering a certificate)
    let is_cert_registration = tx.ops.iter().any(|op| {
        matches!(op, seloria_core::Op::AgentCertRegister { .. })
    });

    if !is_cert_registration {
        if !state.is_certified_agent(&tx.sender_pubkey, current_time) {
            return ValidationResult::err(VmError::AgentNotCertified);
        }

        // Check capabilities for each operation
        if let Some(cert) = state.get_agent(&tx.sender_pubkey) {
            for op in &tx.ops {
                if let Some(required_cap) = get_required_capability(op) {
                    if !cert.has_capability(required_cap) {
                        return ValidationResult::err(VmError::MissingCapability(required_cap));
                    }
                }
            }
        }
    }

    // 3. Verify nonce
    let account_nonce = state
        .get_account(&tx.sender_pubkey)
        .map_or(0, |a| a.nonce);
    let expected_nonce = account_nonce + 1;

    if tx.nonce != expected_nonce {
        return ValidationResult::err(VmError::InvalidNonce {
            expected: expected_nonce,
            got: tx.nonce,
        });
    }

    // 4. Calculate required balances and check
    let mut token_spend: BTreeMap<Hash, u64> = BTreeMap::new();
    if tx.fee > 0 {
        *token_spend.entry(NATIVE_TOKEN_ID).or_insert(0) += tx.fee;
    }

    for op in &tx.ops {
        match op {
            Op::Transfer { amount, .. } => {
                *token_spend.entry(NATIVE_TOKEN_ID).or_insert(0) += *amount;
            }
            Op::ClaimCreate { stake, .. } => {
                *token_spend.entry(NATIVE_TOKEN_ID).or_insert(0) += *stake;
            }
            Op::Attest { stake, .. } => {
                *token_spend.entry(NATIVE_TOKEN_ID).or_insert(0) += *stake;
            }
            Op::TokenTransfer { token_id, amount, .. } => {
                if *token_id != NATIVE_TOKEN_ID && state.get_token(token_id).is_none() {
                    return ValidationResult::err(VmError::TokenNotFound(token_id.to_hex()));
                }
                *token_spend.entry(*token_id).or_insert(0) += *amount;
            }
            Op::PoolCreate {
                token_a,
                token_b,
                amount_a,
                amount_b,
            } => {
                if *token_a != NATIVE_TOKEN_ID && state.get_token(token_a).is_none() {
                    return ValidationResult::err(VmError::TokenNotFound(token_a.to_hex()));
                }
                if *token_b != NATIVE_TOKEN_ID && state.get_token(token_b).is_none() {
                    return ValidationResult::err(VmError::TokenNotFound(token_b.to_hex()));
                }
                *token_spend.entry(*token_a).or_insert(0) += *amount_a;
                *token_spend.entry(*token_b).or_insert(0) += *amount_b;
            }
            Op::PoolAdd {
                pool_id,
                amount_a,
                amount_b,
                ..
            } => {
                let pool = match state.get_pool(pool_id) {
                    Some(pool) => pool,
                    None => return ValidationResult::err(VmError::PoolNotFound(pool_id.to_hex())),
                };
                *token_spend.entry(pool.token_a).or_insert(0) += *amount_a;
                *token_spend.entry(pool.token_b).or_insert(0) += *amount_b;
            }
            Op::PoolRemove { pool_id, lp_amount, .. } => {
                if state.get_pool(pool_id).is_none() {
                    return ValidationResult::err(VmError::PoolNotFound(pool_id.to_hex()));
                }
                let lp_balance = state.get_lp_balance(pool_id, &tx.sender_pubkey);
                if lp_balance < *lp_amount {
                    return ValidationResult::err(VmError::InsufficientBalance {
                        have: lp_balance,
                        need: *lp_amount,
                    });
                }
            }
            Op::Swap {
                pool_id,
                token_in,
                amount_in,
                ..
            } => {
                let pool = match state.get_pool(pool_id) {
                    Some(pool) => pool,
                    None => return ValidationResult::err(VmError::PoolNotFound(pool_id.to_hex())),
                };
                if *token_in != pool.token_a && *token_in != pool.token_b {
                    return ValidationResult::err(VmError::InvalidOperation(
                        "Token not in pool".to_string(),
                    ));
                }
                *token_spend.entry(*token_in).or_insert(0) += *amount_in;
            }
            _ => {}
        }
    }

    for (token_id, required) in &token_spend {
        let available = state.get_token_balance(&tx.sender_pubkey, token_id);
        if available < *required {
            return ValidationResult::err(VmError::InsufficientBalance {
                have: available,
                need: *required,
            });
        }
    }

    let required_balance = token_spend.get(&NATIVE_TOKEN_ID).copied().unwrap_or(0);
    ValidationResult::ok(required_balance)
}

/// Get the required capability for an operation
fn get_required_capability(op: &seloria_core::Op) -> Option<Capability> {
    use seloria_core::Op;

    match op {
        Op::AgentCertRegister { .. } => None, // No capability needed to register
        Op::Transfer { .. } => Some(Capability::TxSubmit),
        Op::TokenCreate { .. } => Some(Capability::TxSubmit),
        Op::TokenTransfer { .. } => Some(Capability::TxSubmit),
        Op::ClaimCreate { .. } => Some(Capability::Claim),
        Op::Attest { .. } => Some(Capability::Attest),
        Op::AppRegister { .. } => Some(Capability::TxSubmit),
        Op::KvPut { .. } | Op::KvDel { .. } | Op::KvAppend { .. } => Some(Capability::KvWrite),
        Op::NamespaceCreate { .. } => Some(Capability::TxSubmit),
        Op::PoolCreate { .. }
        | Op::PoolAdd { .. }
        | Op::PoolRemove { .. }
        | Op::Swap { .. } => Some(Capability::TxSubmit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{
        AgentCertificate, Capability, Hash, KeyPair, Op, SignedAgentCertificate, Transaction,
        hash_blake3,
    };
    use seloria_state::MemoryStorage;

    fn setup_test_state() -> (ChainState<MemoryStorage>, KeyPair, KeyPair) {
        let mut state = ChainState::new(MemoryStorage::new());
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();

        // Add trusted issuer
        state.trusted_issuers.insert(issuer.public);

        // Give agent some balance
        state.credit_token(&agent.public, &seloria_core::NATIVE_TOKEN_ID, 10000);

        // Create and register agent certificate
        let cert = AgentCertificate::new(
            hash_blake3(issuer.public.as_bytes()),
            agent.public,
            0,
            1_000_000, // Expires far in future
            vec![
                Capability::TxSubmit,
                Capability::Claim,
                Capability::Attest,
                Capability::KvWrite,
            ],
            Hash::ZERO,
        );
        let signed_cert = SignedAgentCertificate::new(cert, &issuer.secret).unwrap();
        state.register_agent(signed_cert);

        (state, issuer, agent)
    }

    #[test]
    fn test_validate_valid_transaction() {
        let (state, _, agent) = setup_test_state();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            agent.public,
            1,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 500,
            }],
            &agent.secret,
        )
        .unwrap();

        let result = validate_transaction(&tx, &state, 100);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_invalid_signature() {
        let (state, _, agent) = setup_test_state();
        let wrong_signer = KeyPair::generate();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            agent.public,
            1,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 500,
            }],
            &wrong_signer.secret, // Wrong signer!
        )
        .unwrap();

        let result = validate_transaction(&tx, &state, 100);
        assert!(!result.is_valid);
        assert!(matches!(result.error, Some(VmError::InvalidSignature)));
    }

    #[test]
    fn test_validate_invalid_nonce() {
        let (state, _, agent) = setup_test_state();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            agent.public,
            5, // Wrong nonce (should be 1)
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 500,
            }],
            &agent.secret,
        )
        .unwrap();

        let result = validate_transaction(&tx, &state, 100);
        assert!(!result.is_valid);
        assert!(matches!(result.error, Some(VmError::InvalidNonce { .. })));
    }

    #[test]
    fn test_validate_insufficient_balance() {
        let (state, _, agent) = setup_test_state();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            agent.public,
            1,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 1_000_000, // More than available
            }],
            &agent.secret,
        )
        .unwrap();

        let result = validate_transaction(&tx, &state, 100);
        assert!(!result.is_valid);
        assert!(matches!(
            result.error,
            Some(VmError::InsufficientBalance { .. })
        ));
    }

    #[test]
    fn test_validate_uncertified_agent() {
        let state = ChainState::new(MemoryStorage::new());
        let uncertified = KeyPair::generate();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            uncertified.public,
            1,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 500,
            }],
            &uncertified.secret,
        )
        .unwrap();

        let result = validate_transaction(&tx, &state, 100);
        assert!(!result.is_valid);
        assert!(matches!(result.error, Some(VmError::AgentNotCertified)));
    }
}
