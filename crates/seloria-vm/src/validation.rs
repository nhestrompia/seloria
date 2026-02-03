use seloria_core::{Capability, Transaction};
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

    // 4. Calculate required balance and check
    let required_balance = tx.estimated_cost();
    let available_balance = state.get_balance(&tx.sender_pubkey);

    if available_balance < required_balance {
        return ValidationResult::err(VmError::InsufficientBalance {
            have: available_balance,
            need: required_balance,
        });
    }

    ValidationResult::ok(required_balance)
}

/// Get the required capability for an operation
fn get_required_capability(op: &seloria_core::Op) -> Option<Capability> {
    use seloria_core::Op;

    match op {
        Op::AgentCertRegister { .. } => None, // No capability needed to register
        Op::Transfer { .. } => Some(Capability::TxSubmit),
        Op::ClaimCreate { .. } => Some(Capability::Claim),
        Op::Attest { .. } => Some(Capability::Attest),
        Op::AppRegister { .. } => Some(Capability::TxSubmit),
        Op::KvPut { .. } | Op::KvDel { .. } | Op::KvAppend { .. } => Some(Capability::KvWrite),
        Op::NamespaceCreate { .. } => Some(Capability::TxSubmit),
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
        state.get_or_create_account(&agent.public).balance = 10000;

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
