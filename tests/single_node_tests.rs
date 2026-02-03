//! Single node integration tests

use seloria_core::{
    hash_blake3, AgentCertificate, Capability, GenesisConfig, Hash, KeyPair, Op,
    SignedAgentCertificate, Transaction,
};
use seloria_mempool::{Mempool, MempoolConfig};
use seloria_state::{ChainState, MemoryStorage};
use seloria_vm::Executor;

/// Test helper to set up a test environment
fn setup_test_env() -> (ChainState<MemoryStorage>, KeyPair, KeyPair, KeyPair) {
    let mut state = ChainState::new(MemoryStorage::new());
    let issuer = KeyPair::generate();
    let agent = KeyPair::generate();
    let validator = KeyPair::generate();

    let config = GenesisConfig {
        chain_id: 1,
        timestamp: 0,
        initial_balances: vec![(agent.public, 1_000_000_000)],
        trusted_issuers: vec![issuer.public],
        validators: vec![validator.public],
    };

    state.init_genesis(&config).unwrap();

    // Register agent
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

    (state, issuer, agent, validator)
}

#[test]
fn test_genesis_initialization() {
    let (state, issuer, agent, validator) = setup_test_env();

    // Verify genesis state
    assert_eq!(state.chain_id, 1);
    assert_eq!(state.height, 0);
    assert!(state.is_trusted_issuer(&issuer.public));
    assert_eq!(state.validators.len(), 1);
    assert_eq!(state.validators[0], validator.public);

    // Verify initial balance
    assert_eq!(state.get_balance(&agent.public), 1_000_000_000);
}

#[test]
fn test_agent_certification() {
    let (mut state, issuer, _, _) = setup_test_env();

    // Register a new agent
    let new_agent = KeyPair::generate();
    let cert = AgentCertificate::new(
        hash_blake3(issuer.public.as_bytes()),
        new_agent.public,
        0,
        1_000_000,
        vec![Capability::TxSubmit],
        Hash::ZERO,
    );
    let signed_cert = SignedAgentCertificate::new(cert, &issuer.secret).unwrap();
    state.register_agent(signed_cert);

    // Verify certification
    assert!(state.is_certified_agent(&new_agent.public, 500));
    assert!(!state.is_certified_agent(&new_agent.public, 2_000_000));
}

#[test]
fn test_transfer_transaction() {
    let (mut state, _, agent, _) = setup_test_env();
    let receiver = KeyPair::generate();

    let tx = Transaction::new_signed(
        agent.public,
        1,
        1000, // fee
        vec![Op::Transfer {
            to: receiver.public,
            amount: 100_000,
        }],
        &agent.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    let result = executor.execute_transaction(&tx, &mut state);

    assert!(result.success, "Transaction should succeed");
    assert_eq!(state.get_balance(&receiver.public), 100_000);
    assert_eq!(
        state.get_balance(&agent.public),
        1_000_000_000 - 100_000 - 1000
    );
}

#[test]
fn test_multiple_transfers() {
    let (mut state, _, agent, _) = setup_test_env();

    let executor = Executor::new(100, 1);

    // Execute multiple transfers
    for i in 1..=5 {
        let receiver = KeyPair::generate();
        let tx = Transaction::new_signed(
            agent.public,
            i,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 1000,
            }],
            &agent.secret,
        )
        .unwrap();

        let result = executor.execute_transaction(&tx, &mut state);
        assert!(result.success, "Transfer {} should succeed", i);
    }

    // Verify nonce
    let account = state.get_account(&agent.public).unwrap();
    assert_eq!(account.nonce, 5);
}

#[test]
fn test_nonce_enforcement() {
    let (mut state, _, agent, _) = setup_test_env();
    let receiver = KeyPair::generate();

    let executor = Executor::new(100, 1);

    // First transaction with correct nonce (1)
    let tx1 = Transaction::new_signed(
        agent.public,
        1,
        100,
        vec![Op::Transfer {
            to: receiver.public,
            amount: 1000,
        }],
        &agent.secret,
    )
    .unwrap();
    assert!(executor.execute_transaction(&tx1, &mut state).success);

    // Try to replay same transaction (nonce 1)
    let tx_replay = Transaction::new_signed(
        agent.public,
        1,
        100,
        vec![Op::Transfer {
            to: receiver.public,
            amount: 1000,
        }],
        &agent.secret,
    )
    .unwrap();
    assert!(!executor.execute_transaction(&tx_replay, &mut state).success);

    // Skip nonce (should fail)
    let tx_skip = Transaction::new_signed(
        agent.public,
        5,
        100,
        vec![Op::Transfer {
            to: receiver.public,
            amount: 1000,
        }],
        &agent.secret,
    )
    .unwrap();
    assert!(!executor.execute_transaction(&tx_skip, &mut state).success);
}

#[test]
fn test_insufficient_balance() {
    let (mut state, _, agent, _) = setup_test_env();
    let receiver = KeyPair::generate();

    let tx = Transaction::new_signed(
        agent.public,
        1,
        100,
        vec![Op::Transfer {
            to: receiver.public,
            amount: 2_000_000_000, // More than balance
        }],
        &agent.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    let result = executor.execute_transaction(&tx, &mut state);

    assert!(!result.success);
    assert!(result.error.unwrap().contains("Insufficient balance"));
}

#[test]
fn test_uncertified_agent_rejected() {
    let (mut state, _, _, _) = setup_test_env();
    let uncertified = KeyPair::generate();
    let receiver = KeyPair::generate();

    // Give the uncertified agent some balance
    state.get_or_create_account(&uncertified.public).balance = 100_000;

    let tx = Transaction::new_signed(
        uncertified.public,
        1,
        100,
        vec![Op::Transfer {
            to: receiver.public,
            amount: 1000,
        }],
        &uncertified.secret,
    )
    .unwrap();

    let executor = Executor::new(100, 1);
    let result = executor.execute_transaction(&tx, &mut state);

    assert!(!result.success);
    assert!(result.error.unwrap().contains("not certified"));
}

#[tokio::test]
async fn test_mempool_operations() {
    let mempool = Mempool::new(MempoolConfig::default());
    let sender = KeyPair::generate();

    // Add transactions
    for i in 1..=3 {
        let tx = Transaction::new_signed(
            sender.public,
            i,
            100 * i,
            vec![Op::Transfer {
                to: KeyPair::generate().public,
                amount: 1000,
            }],
            &sender.secret,
        )
        .unwrap();

        mempool.add(tx).await.unwrap();
    }

    assert_eq!(mempool.size().await, 3);

    // Get ordered transactions
    let txs = mempool.get_transactions(10).await;
    assert_eq!(txs.len(), 3);

    // Highest fee should be first (300 > 200 > 100)
    assert_eq!(txs[0].fee, 300);
}
