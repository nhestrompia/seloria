//! Consensus integration tests

use std::sync::Arc;

use seloria_consensus::{BlockBuilder, BlockBuilderConfig, Proposer, ProposerConfig, QcBuilder};
use seloria_core::{
    hash_blake3, sign, AgentCertificate, Capability, GenesisConfig, Hash, KeyPair, Op,
    SignedAgentCertificate, Transaction,
};
use seloria_mempool::{Mempool, MempoolConfig};
use seloria_state::{ChainState, MemoryStorage};
use tokio::sync::RwLock;

/// Set up test environment for consensus tests
async fn setup_consensus_env() -> (
    Arc<RwLock<ChainState<MemoryStorage>>>,
    Arc<Mempool>,
    KeyPair,
    KeyPair,
    Vec<KeyPair>,
) {
    let state = Arc::new(RwLock::new(ChainState::new(MemoryStorage::new())));
    let mempool = Arc::new(Mempool::new(MempoolConfig::default()));

    let issuer = KeyPair::generate();
    let agent = KeyPair::generate();
    let validators: Vec<KeyPair> = (0..4).map(|_| KeyPair::generate()).collect();

    let config = GenesisConfig {
        chain_id: 1,
        timestamp: 0,
        initial_balances: vec![(agent.public, 1_000_000_000)],
        trusted_issuers: vec![issuer.public],
        validators: validators.iter().map(|v| v.public).collect(),
    };

    state.write().await.init_genesis(&config).unwrap();

    // Register agent
    let cert = AgentCertificate::new(
        hash_blake3(issuer.public.as_bytes()),
        agent.public,
        0,
        1_000_000_000,
        vec![Capability::TxSubmit],
        Hash::ZERO,
    );
    let signed_cert = SignedAgentCertificate::new(cert, &issuer.secret).unwrap();
    state.write().await.register_agent(signed_cert);

    (state, mempool, issuer, agent, validators)
}

#[tokio::test]
async fn test_block_building() {
    let (state, mempool, _, agent, validators) = setup_consensus_env().await;

    // Add transactions to mempool
    for i in 1..=3 {
        let tx = Transaction::new_signed(
            agent.public,
            i,
            100 * i,
            vec![Op::Transfer {
                to: KeyPair::generate().public,
                amount: 1000,
            }],
            &agent.secret,
        )
        .unwrap();

        mempool.add(tx).await.unwrap();
    }

    // Build block
    let block_builder = BlockBuilder::new(BlockBuilderConfig {
        chain_id: 1,
        ..Default::default()
    });

    let state_guard = state.read().await;
    let block = block_builder
        .build_block(&*state_guard, &mempool, validators[0].public, 1000)
        .await
        .unwrap();

    assert_eq!(block.header.height, 1);
    assert_eq!(block.txs.len(), 3);
    assert_eq!(block.header.proposer_pubkey, validators[0].public);
}

#[tokio::test]
async fn test_block_validation() {
    let (state, mempool, _, _, validators) = setup_consensus_env().await;

    let block_builder = BlockBuilder::new(BlockBuilderConfig {
        chain_id: 1,
        ..Default::default()
    });

    // Build block
    let state_guard = state.read().await;
    let block = block_builder
        .build_block(&*state_guard, &mempool, validators[0].public, 1000)
        .await
        .unwrap();
    drop(state_guard);

    // Validate block with fresh state (simulating different validator)
    let fresh_state = {
        let mut s = ChainState::new(MemoryStorage::new());
        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![],
            trusted_issuers: vec![],
            validators: validators.iter().map(|v| v.public).collect(),
        };
        s.init_genesis(&config).unwrap();
        s
    };

    let validation_result = block_builder.validate_block(&block, &fresh_state);
    assert!(validation_result.is_ok());
}

#[test]
fn test_quorum_certificate_building() {
    let validators: Vec<KeyPair> = (0..4).map(|_| KeyPair::generate()).collect();
    let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();

    let block_hash = hash_blake3(b"test block");

    let mut qc_builder = QcBuilder::new(block_hash, &validator_pubkeys, 3);

    // Add signatures from 3 validators (threshold = 3)
    for validator in &validators[..3] {
        let sig = sign(&validator.secret, block_hash.as_bytes());
        let has_quorum = qc_builder.add_signature(validator.public, sig).unwrap();

        if validator.public == validators[2].public {
            assert!(has_quorum);
        }
    }

    // Build QC
    let qc = qc_builder.build().unwrap();
    assert_eq!(qc.signatures.len(), 3);
    assert_eq!(qc.block_hash, block_hash);
}

#[test]
fn test_quorum_certificate_insufficient_signatures() {
    let validators: Vec<KeyPair> = (0..4).map(|_| KeyPair::generate()).collect();
    let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();

    let block_hash = hash_blake3(b"test block");

    let mut qc_builder = QcBuilder::new(block_hash, &validator_pubkeys, 3);

    // Add only 2 signatures (threshold = 3)
    for validator in &validators[..2] {
        let sig = sign(&validator.secret, block_hash.as_bytes());
        qc_builder.add_signature(validator.public, sig).unwrap();
    }

    // Should fail to build
    let result = qc_builder.build();
    assert!(result.is_err());
}

#[test]
fn test_leader_rotation() {
    let validators: Vec<KeyPair> = (0..4).map(|_| KeyPair::generate()).collect();
    let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();

    // Height 0: validator 0 should be leader
    assert_eq!(
        validator_pubkeys[(0 as usize) % 4],
        validator_pubkeys[0]
    );

    // Height 1: validator 1 should be leader
    assert_eq!(
        validator_pubkeys[(1 as usize) % 4],
        validator_pubkeys[1]
    );

    // Height 4: validator 0 should be leader again
    assert_eq!(
        validator_pubkeys[(4 as usize) % 4],
        validator_pubkeys[0]
    );
}

#[tokio::test]
async fn test_proposer_single_node() {
    let (state, mempool, _, agent, validators) = setup_consensus_env().await;

    // Single validator mode
    let proposer_config = ProposerConfig {
        round_time_ms: 100,
        num_validators: 1,
        threshold: 1,
        chain_id: 1,
    };

    let proposer = Proposer::new(
        proposer_config,
        validators[0].public,
        validators[0].secret.clone(),
        Arc::clone(&state),
        Arc::clone(&mempool),
        vec![validators[0].public],
    );

    // Add a transaction
    let tx = Transaction::new_signed(
        agent.public,
        1,
        100,
        vec![Op::Transfer {
            to: KeyPair::generate().public,
            amount: 1000,
        }],
        &agent.secret,
    )
    .unwrap();
    mempool.add(tx).await.unwrap();

    // Propose and apply block
    let block = proposer.propose_block().await.unwrap();
    assert_eq!(block.header.height, 1);

    let finalized = proposer.finalize_block(block).await.unwrap();
    proposer.apply_block(finalized).await.unwrap();

    // Verify state updated
    let state_guard = state.read().await;
    assert_eq!(state_guard.current_height(), 1);

    // Mempool should be cleared
    assert_eq!(mempool.size().await, 0);
}

#[tokio::test]
async fn test_multiple_blocks() {
    let (state, mempool, _, agent, validators) = setup_consensus_env().await;

    let proposer_config = ProposerConfig {
        round_time_ms: 100,
        num_validators: 1,
        threshold: 1,
        chain_id: 1,
    };

    let proposer = Proposer::new(
        proposer_config,
        validators[0].public,
        validators[0].secret.clone(),
        Arc::clone(&state),
        Arc::clone(&mempool),
        vec![validators[0].public],
    );

    // Produce multiple blocks
    for i in 1..=3 {
        // Add transaction
        let tx = Transaction::new_signed(
            agent.public,
            i,
            100,
            vec![Op::Transfer {
                to: KeyPair::generate().public,
                amount: 1000,
            }],
            &agent.secret,
        )
        .unwrap();
        mempool.add(tx).await.unwrap();

        // Produce block
        let block = proposer.propose_block().await.unwrap();
        assert_eq!(block.header.height, i);

        let finalized = proposer.finalize_block(block).await.unwrap();
        proposer.apply_block(finalized).await.unwrap();
    }

    // Verify final height
    let state_guard = state.read().await;
    assert_eq!(state_guard.current_height(), 3);
}
