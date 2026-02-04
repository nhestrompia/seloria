use seloria_core::{
    calculate_settlement, hash_blake3, Attestation, Claim, ClaimStatus, Hash, LockId, PublicKey,
    Vote, NATIVE_TOKEN_ID,
};
use seloria_state::{ChainState, Storage};
use tracing::{debug, info};

use crate::error::VmError;

/// Execute CLAIM_CREATE operation
pub fn execute_claim_create<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    claim_type: &str,
    payload_hash: &Hash,
    stake: u64,
    block_height: u64,
) -> Result<Hash, VmError> {
    // Check sender has sufficient balance for stake
    let sender_balance = state.get_balance(sender);
    if sender_balance < stake {
        return Err(VmError::InsufficientBalance {
            have: sender_balance,
            need: stake,
        });
    }

    // Generate claim ID
    let claim_id = generate_claim_id(sender, claim_type, payload_hash, block_height);

    // Create lock ID for this claim
    let lock_id = LockId::new(claim_id);

    // Lock the stake
    state.lock_stake(sender, lock_id, stake)?;

    // Create the claim
    let claim = Claim::new(
        claim_id,
        claim_type.to_string(),
        *payload_hash,
        *sender,
        stake,
        block_height,
    );

    state.add_claim(claim);

    debug!(
        "Created claim {} of type '{}' with stake {}",
        claim_id, claim_type, stake
    );

    Ok(claim_id)
}

/// Execute ATTEST operation
pub fn execute_attest<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    claim_id: &Hash,
    vote: Vote,
    stake: u64,
    block_height: u64,
) -> Result<bool, VmError> {
    // Check claim exists
    if state.get_claim(claim_id).is_none() {
        return Err(VmError::ClaimNotFound(claim_id.to_hex()));
    }

    // Check claim is still pending
    {
        let claim = state.get_claim(claim_id).unwrap();
        if claim.status != ClaimStatus::Pending {
            return Err(VmError::ClaimAlreadyFinalized);
        }

        // Check sender hasn't already attested
        if claim.has_attested(sender) {
            return Err(VmError::AlreadyAttested);
        }
    }

    // Check sender has sufficient balance for stake
    let sender_balance = state.get_balance(sender);
    if sender_balance < stake {
        return Err(VmError::InsufficientBalance {
            have: sender_balance,
            need: stake,
        });
    }

    // Create lock ID for this attestation
    let lock_id = create_attestation_lock_id(claim_id, sender);

    // Lock the stake
    state.lock_stake(sender, lock_id, stake)?;

    // Add attestation to claim
    let attestation = Attestation {
        attester: *sender,
        vote,
        stake,
        block_height,
    };

    let claim = state.get_claim_mut(claim_id).unwrap();
    claim.add_attestation(attestation);

    debug!(
        "Added attestation to claim {} from {} with vote {:?} and stake {}",
        claim_id, sender, vote, stake
    );

    // Check if claim should finalize
    let finalized = claim.try_finalize();

    if finalized {
        info!("Claim {} finalized with status {:?}", claim_id, claim.status);
        // Settlement will be handled separately
        settle_claim(state, claim_id)?;
    }

    Ok(finalized)
}

/// Settle a finalized claim
pub fn settle_claim<S: Storage>(state: &mut ChainState<S>, claim_id: &Hash) -> Result<(), VmError> {
    let claim = state
        .get_claim(claim_id)
        .ok_or_else(|| VmError::ClaimNotFound(claim_id.to_hex()))?
        .clone();

    if claim.status == ClaimStatus::Pending {
        return Err(VmError::InvalidOperation(
            "Cannot settle pending claim".to_string(),
        ));
    }

    // Calculate settlement
    let settlement = calculate_settlement(&claim).ok_or_else(|| {
        VmError::InvalidOperation("Failed to calculate settlement".to_string())
    })?;

    // Apply settlement
    let settlement_count = settlement.len();
    for (pubkey, change) in settlement.iter() {
        // First, remove locks (they were tracking the stake)
        // For creator
        if *pubkey == claim.creator {
            let lock_id = LockId::new(*claim_id);
            // Don't unlock - we'll handle the balance change directly
            let account = state.get_or_create_account(pubkey);
            account.locked.remove(&lock_id);
        } else {
            // For attesters
            let lock_id = create_attestation_lock_id(claim_id, pubkey);
            let account = state.get_or_create_account(pubkey);
            account.locked.remove(&lock_id);
        }

        // Apply balance change (settlement already calculated the final amounts)
        if *change >= 0 {
            let account = state.get_or_create_account(pubkey);
            account.credit(&NATIVE_TOKEN_ID, *change as u64);
        } else {
            let amount = (-*change) as u64;
            state.debit_token(pubkey, &NATIVE_TOKEN_ID, amount)?;
        }

        debug!(
            "Settlement: {} balance adjusted by {}",
            pubkey, change
        );
    }

    info!(
        "Settled claim {} - {} participants affected",
        claim_id,
        settlement_count
    );

    Ok(())
}

/// Generate a unique claim ID
fn generate_claim_id(
    creator: &PublicKey,
    claim_type: &str,
    payload_hash: &Hash,
    block_height: u64,
) -> Hash {
    let mut data = Vec::new();
    data.extend_from_slice(creator.as_bytes());
    data.extend_from_slice(claim_type.as_bytes());
    data.extend_from_slice(payload_hash.as_bytes());
    data.extend_from_slice(&block_height.to_le_bytes());
    hash_blake3(&data)
}

/// Create a lock ID for an attestation
fn create_attestation_lock_id(claim_id: &Hash, attester: &PublicKey) -> LockId {
    let mut data = Vec::new();
    data.extend_from_slice(claim_id.as_bytes());
    data.extend_from_slice(attester.as_bytes());
    LockId::new(hash_blake3(&data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::KeyPair;
    use seloria_state::MemoryStorage;

    fn setup_state_with_agents() -> (ChainState<MemoryStorage>, KeyPair, KeyPair, KeyPair) {
        let state = ChainState::new(MemoryStorage::new());
        let creator = KeyPair::generate();
        let attester1 = KeyPair::generate();
        let attester2 = KeyPair::generate();
        (state, creator, attester1, attester2)
    }

    #[test]
    fn test_claim_create() {
        let (mut state, creator, _, _) = setup_state_with_agents();
        state.credit_token(&creator.public, &NATIVE_TOKEN_ID, 10000);

        let payload_hash = hash_blake3(b"test payload");
        let claim_id = execute_claim_create(
            &mut state,
            &creator.public,
            "test",
            &payload_hash,
            1000,
            1,
        )
        .unwrap();

        // Check claim was created
        let claim = state.get_claim(&claim_id).unwrap();
        assert_eq!(claim.creator, creator.public);
        assert_eq!(claim.creator_stake, 1000);
        assert_eq!(claim.yes_stake, 1000);
        assert_eq!(claim.status, ClaimStatus::Pending);

        // Check stake was locked
        assert_eq!(state.get_balance(&creator.public), 9000);
    }

    #[test]
    fn test_attestation() {
        let (mut state, creator, attester, _) = setup_state_with_agents();
        state.credit_token(&creator.public, &NATIVE_TOKEN_ID, 10000);
        state.credit_token(&attester.public, &NATIVE_TOKEN_ID, 10000);

        let payload_hash = hash_blake3(b"test payload");
        let claim_id = execute_claim_create(
            &mut state,
            &creator.public,
            "test",
            &payload_hash,
            1000,
            1,
        )
        .unwrap();

        execute_attest(&mut state, &attester.public, &claim_id, Vote::No, 500, 2).unwrap();

        let claim = state.get_claim(&claim_id).unwrap();
        assert_eq!(claim.no_stake, 500);
        assert_eq!(claim.attestations.len(), 1);
    }

    #[test]
    fn test_claim_finalization_yes() {
        let (mut state, creator, attester, _) = setup_state_with_agents();
        state.credit_token(&creator.public, &NATIVE_TOKEN_ID, 10000);
        state.credit_token(&attester.public, &NATIVE_TOKEN_ID, 10000);

        let payload_hash = hash_blake3(b"test payload");
        let claim_id = execute_claim_create(
            &mut state,
            &creator.public,
            "test",
            &payload_hash,
            1000,
            1,
        )
        .unwrap();

        // Need 2x creator_stake = 2000 for YES
        // Creator already has 1000, need 1000 more
        let finalized =
            execute_attest(&mut state, &attester.public, &claim_id, Vote::Yes, 1000, 2).unwrap();

        assert!(finalized);
        let claim = state.get_claim(&claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::FinalizedYes);
    }

    #[test]
    fn test_claim_finalization_no() {
        let (mut state, creator, attester1, attester2) = setup_state_with_agents();
        state.credit_token(&creator.public, &NATIVE_TOKEN_ID, 10000);
        state.credit_token(&attester1.public, &NATIVE_TOKEN_ID, 10000);
        state.credit_token(&attester2.public, &NATIVE_TOKEN_ID, 10000);

        let payload_hash = hash_blake3(b"test payload");
        let claim_id = execute_claim_create(
            &mut state,
            &creator.public,
            "test",
            &payload_hash,
            1000,
            1,
        )
        .unwrap();

        // Need 2x creator_stake = 2000 for NO
        execute_attest(&mut state, &attester1.public, &claim_id, Vote::No, 1000, 2).unwrap();
        let finalized =
            execute_attest(&mut state, &attester2.public, &claim_id, Vote::No, 1000, 3).unwrap();

        assert!(finalized);
        let claim = state.get_claim(&claim_id).unwrap();
        assert_eq!(claim.status, ClaimStatus::FinalizedNo);
    }

    #[test]
    fn test_double_attestation() {
        let (mut state, creator, attester, _) = setup_state_with_agents();
        state.credit_token(&creator.public, &NATIVE_TOKEN_ID, 10000);
        state.credit_token(&attester.public, &NATIVE_TOKEN_ID, 10000);

        let payload_hash = hash_blake3(b"test payload");
        let claim_id = execute_claim_create(
            &mut state,
            &creator.public,
            "test",
            &payload_hash,
            1000,
            1,
        )
        .unwrap();

        execute_attest(&mut state, &attester.public, &claim_id, Vote::No, 500, 2).unwrap();
        let result = execute_attest(&mut state, &attester.public, &claim_id, Vote::Yes, 500, 3);

        assert!(matches!(result, Err(VmError::AlreadyAttested)));
    }
}
