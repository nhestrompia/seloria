use seloria_core::PublicKey;
use seloria_state::{ChainState, Storage};
use tracing::debug;

use crate::error::VmError;

/// Execute TRANSFER operation
pub fn execute_transfer<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    to: &PublicKey,
    amount: u64,
) -> Result<(), VmError> {
    // Check sender has sufficient balance
    let sender_balance = state.get_balance(sender);
    if sender_balance < amount {
        return Err(VmError::InsufficientBalance {
            have: sender_balance,
            need: amount,
        });
    }

    // Perform transfer
    state.transfer(sender, to, amount)?;

    debug!("Transferred {} from {} to {}", amount, sender, to);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::KeyPair;
    use seloria_state::MemoryStorage;

    #[test]
    fn test_transfer_success() {
        let mut state = ChainState::new(MemoryStorage::new());
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        state.credit_token(&alice.public, &seloria_core::NATIVE_TOKEN_ID, 1000);

        execute_transfer(&mut state, &alice.public, &bob.public, 300).unwrap();

        assert_eq!(state.get_balance(&alice.public), 700);
        assert_eq!(state.get_balance(&bob.public), 300);
    }

    #[test]
    fn test_transfer_insufficient_balance() {
        let mut state = ChainState::new(MemoryStorage::new());
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        state.credit_token(&alice.public, &seloria_core::NATIVE_TOKEN_ID, 100);

        let result = execute_transfer(&mut state, &alice.public, &bob.public, 500);
        assert!(matches!(result, Err(VmError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_transfer_to_self() {
        let mut state = ChainState::new(MemoryStorage::new());
        let alice = KeyPair::generate();

        state.credit_token(&alice.public, &seloria_core::NATIVE_TOKEN_ID, 1000);

        execute_transfer(&mut state, &alice.public, &alice.public, 300).unwrap();

        // Balance should remain the same
        assert_eq!(state.get_balance(&alice.public), 1000);
    }
}
