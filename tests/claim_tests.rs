//! Claim system integration tests

use seloria_core::{
    hash_blake3, AgentCertificate, Capability, ClaimStatus, GenesisConfig, Hash, KeyPair, Op,
    SignedAgentCertificate, Transaction, Vote,
};
use seloria_state::{ChainState, MemoryStorage};
use seloria_vm::Executor;

/// Set up test environment with multiple agents
fn setup_multi_agent_env() -> (
    ChainState<MemoryStorage>,
    KeyPair,
    KeyPair,
    KeyPair,
    KeyPair,
    KeyPair,
) {
    let mut state = ChainState::new(MemoryStorage::new());
    let issuer = KeyPair::generate();
    let creator = KeyPair::generate();
    let attester1 = KeyPair::generate();
    let attester2 = KeyPair::generate();
    let validator = KeyPair::generate();

    let config = GenesisConfig {
        chain_id: 1,
        timestamp: 0,
        initial_balances: vec![
            (creator.public, 100_000_000),
            (attester1.public, 100_000_000),
            (attester2.public, 100_000_000),
        ],
        trusted_issuers: vec![issuer.public],
        validators: vec![validator.public],
    };

    state.init_genesis(&config).unwrap();

    // Register all agents
    for agent in [&creator, &attester1, &attester2] {
        let cert = AgentCertificate::new(
            hash_blake3(issuer.public.as_bytes()),
            agent.public,
            0,
            1_000_000_000,
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
    }

    (state, issuer, creator, attester1, attester2, validator)
}

#[test]
fn test_claim_creation() {
    let (mut state, _, creator, _, _, _) = setup_multi_agent_env();

    let payload_hash = hash_blake3(b"test claim payload");
    let tx = Transaction::new_signed(
        creator.public,
        1,
        1000,
        vec![Op::ClaimCreate {
            claim_type: "price_oracle".to_string(),
            payload_hash,
            stake: 10_000,
        }],
        &creator.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    let result = executor.execute_transaction(&tx, &mut state);

    assert!(result.success, "Claim creation should succeed");

    // Verify stake was locked
    let creator_account = state.get_account(&creator.public).unwrap();
    assert!(creator_account.balance < 100_000_000);
    assert!(creator_account.total_balance() < 100_000_000); // Fee was deducted

    // Verify claim exists
    assert_eq!(state.claims.len(), 1);
    let claim = state.claims.values().next().unwrap();
    assert_eq!(claim.claim_type, "price_oracle");
    assert_eq!(claim.creator_stake, 10_000);
    assert_eq!(claim.yes_stake, 10_000); // Creator implicitly votes YES
    assert_eq!(claim.status, ClaimStatus::Pending);
}

#[test]
fn test_attestation() {
    let (mut state, _, creator, attester1, _, _) = setup_multi_agent_env();

    // Create claim
    let payload_hash = hash_blake3(b"test claim");
    let tx_create = Transaction::new_signed(
        creator.public,
        1,
        1000,
        vec![Op::ClaimCreate {
            claim_type: "test".to_string(),
            payload_hash,
            stake: 10_000,
        }],
        &creator.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    let result = executor.execute_transaction(&tx_create, &mut state);
    assert!(result.success);

    // Get claim ID
    let claim_id = *state.claims.keys().next().unwrap();

    // Attest
    let tx_attest = Transaction::new_signed(
        attester1.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::No,
            stake: 5_000,
        }],
        &attester1.secret,
    )
    .unwrap();

    let result = executor.execute_transaction(&tx_attest, &mut state);
    assert!(result.success, "Attestation should succeed");

    // Verify attestation
    let claim = state.get_claim(&claim_id).unwrap();
    assert_eq!(claim.no_stake, 5_000);
    assert_eq!(claim.attestations.len(), 1);
    assert_eq!(claim.status, ClaimStatus::Pending);
}

#[test]
fn test_claim_finalization_yes() {
    let (mut state, _, creator, attester1, _, _) = setup_multi_agent_env();

    // Create claim with 10_000 stake
    let payload_hash = hash_blake3(b"test claim");
    let tx_create = Transaction::new_signed(
        creator.public,
        1,
        1000,
        vec![Op::ClaimCreate {
            claim_type: "test".to_string(),
            payload_hash,
            stake: 10_000,
        }],
        &creator.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    executor.execute_transaction(&tx_create, &mut state);

    let claim_id = *state.claims.keys().next().unwrap();

    // Need 2x creator_stake = 20_000 for YES finalization
    // Creator has 10_000, need 10_000 more
    let tx_attest = Transaction::new_signed(
        attester1.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::Yes,
            stake: 10_000,
        }],
        &attester1.secret,
    )
    .unwrap();

    let result = executor.execute_transaction(&tx_attest, &mut state);
    assert!(result.success);

    // Verify finalization
    let claim = state.get_claim(&claim_id).unwrap();
    assert_eq!(claim.status, ClaimStatus::FinalizedYes);
    assert_eq!(claim.yes_stake, 20_000);
}

#[test]
fn test_claim_finalization_no() {
    let (mut state, _, creator, attester1, attester2, _) = setup_multi_agent_env();

    // Create claim with 10_000 stake
    let payload_hash = hash_blake3(b"test claim");
    let tx_create = Transaction::new_signed(
        creator.public,
        1,
        1000,
        vec![Op::ClaimCreate {
            claim_type: "test".to_string(),
            payload_hash,
            stake: 10_000,
        }],
        &creator.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    executor.execute_transaction(&tx_create, &mut state);

    let claim_id = *state.claims.keys().next().unwrap();

    // Need 2x creator_stake = 20_000 for NO finalization
    // First attester adds 10_000
    let tx_attest1 = Transaction::new_signed(
        attester1.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::No,
            stake: 10_000,
        }],
        &attester1.secret,
    )
    .unwrap();

    executor.execute_transaction(&tx_attest1, &mut state);

    // Still pending
    assert_eq!(
        state.get_claim(&claim_id).unwrap().status,
        ClaimStatus::Pending
    );

    // Second attester adds 10_000 more
    let tx_attest2 = Transaction::new_signed(
        attester2.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::No,
            stake: 10_000,
        }],
        &attester2.secret,
    )
    .unwrap();

    let result = executor.execute_transaction(&tx_attest2, &mut state);
    assert!(result.success);

    // Verify finalization
    let claim = state.get_claim(&claim_id).unwrap();
    assert_eq!(claim.status, ClaimStatus::FinalizedNo);
    assert_eq!(claim.no_stake, 20_000);
}

#[test]
fn test_double_attestation_rejected() {
    let (mut state, _, creator, attester1, _, _) = setup_multi_agent_env();

    // Create claim
    let payload_hash = hash_blake3(b"test claim");
    let tx_create = Transaction::new_signed(
        creator.public,
        1,
        1000,
        vec![Op::ClaimCreate {
            claim_type: "test".to_string(),
            payload_hash,
            stake: 10_000,
        }],
        &creator.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    executor.execute_transaction(&tx_create, &mut state);

    let claim_id = *state.claims.keys().next().unwrap();

    // First attestation
    let tx_attest1 = Transaction::new_signed(
        attester1.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::No,
            stake: 5_000,
        }],
        &attester1.secret,
    )
    .unwrap();

    let result = executor.execute_transaction(&tx_attest1, &mut state);
    assert!(result.success);

    // Second attestation from same attester
    let tx_attest2 = Transaction::new_signed(
        attester1.public,
        2,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::Yes,
            stake: 5_000,
        }],
        &attester1.secret,
    )
    .unwrap();

    let result = executor.execute_transaction(&tx_attest2, &mut state);
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Already attested"));
}

#[test]
fn test_attestation_to_finalized_claim_rejected() {
    let (mut state, _, creator, attester1, attester2, _) = setup_multi_agent_env();

    // Create claim
    let payload_hash = hash_blake3(b"test claim");
    let tx_create = Transaction::new_signed(
        creator.public,
        1,
        1000,
        vec![Op::ClaimCreate {
            claim_type: "test".to_string(),
            payload_hash,
            stake: 10_000,
        }],
        &creator.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    executor.execute_transaction(&tx_create, &mut state);

    let claim_id = *state.claims.keys().next().unwrap();

    // Finalize claim
    let tx_attest1 = Transaction::new_signed(
        attester1.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::Yes,
            stake: 10_000,
        }],
        &attester1.secret,
    )
    .unwrap();

    executor.execute_transaction(&tx_attest1, &mut state);

    // Claim is now finalized
    assert_eq!(
        state.get_claim(&claim_id).unwrap().status,
        ClaimStatus::FinalizedYes
    );

    // Try to attest to finalized claim
    let tx_attest2 = Transaction::new_signed(
        attester2.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::No,
            stake: 5_000,
        }],
        &attester2.secret,
    )
    .unwrap();

    let result = executor.execute_transaction(&tx_attest2, &mut state);
    assert!(!result.success);
    assert!(result.error.unwrap().contains("already finalized"));
}

#[test]
fn test_claim_settlement() {
    let (mut state, _, creator, attester1, attester2, _) = setup_multi_agent_env();

    // Record initial balances
    let creator_initial = state.get_balance(&creator.public);
    let attester1_initial = state.get_balance(&attester1.public);
    let attester2_initial = state.get_balance(&attester2.public);

    // Create claim with 10_000 stake
    let payload_hash = hash_blake3(b"test claim");
    let tx_create = Transaction::new_signed(
        creator.public,
        1,
        1000,
        vec![Op::ClaimCreate {
            claim_type: "test".to_string(),
            payload_hash,
            stake: 10_000,
        }],
        &creator.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    executor.execute_transaction(&tx_create, &mut state);

    let claim_id = *state.claims.keys().next().unwrap();

    // Attester1 votes YES with 5_000
    let tx_attest1 = Transaction::new_signed(
        attester1.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::Yes,
            stake: 5_000,
        }],
        &attester1.secret,
    )
    .unwrap();

    executor.execute_transaction(&tx_attest1, &mut state);

    // Attester2 votes NO with 20_000 (enough to finalize as NO)
    let tx_attest2 = Transaction::new_signed(
        attester2.public,
        1,
        1000,
        vec![Op::Attest {
            claim_id,
            vote: Vote::No,
            stake: 20_000,
        }],
        &attester2.secret,
    )
    .unwrap();

    executor.execute_transaction(&tx_attest2, &mut state);

    // Claim should be finalized as NO
    let claim = state.get_claim(&claim_id).unwrap();
    assert_eq!(claim.status, ClaimStatus::FinalizedNo);

    // Settlement:
    // Losers (creator + attester1) lose 20% of their stakes
    // Creator: 10_000 * 0.2 = 2_000 slashed
    // Attester1: 5_000 * 0.2 = 1_000 slashed
    // Total slashed: 3_000
    // Winner (attester2) gets 20_000 + 3_000 = 23_000

    // Verify balances changed appropriately
    // Note: The settlement happens immediately when the claim finalizes
    let creator_final = state.get_account(&creator.public).unwrap().total_balance();
    let attester1_final = state.get_account(&attester1.public).unwrap().total_balance();
    let attester2_final = state.get_account(&attester2.public).unwrap().total_balance();

    // Creator lost fee (1000) and 20% of stake (2000)
    assert!(creator_final < creator_initial - 2000);

    // Attester1 lost fee (1000) and 20% of stake (1000)
    assert!(attester1_final < attester1_initial - 1000);

    // Attester2 lost fee (1000) but gained slashed amounts
    // Net should be positive relative to just losing fee
    assert!(attester2_final > attester2_initial - 21000);
}
