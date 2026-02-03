use seloria_core::{AppMeta, Hash, Op, Transaction};
use seloria_state::{ChainState, Storage};
use tracing::{debug, error, info};

use crate::error::VmError;
use crate::opcodes::{
    execute_agent_cert_register, execute_attest, execute_claim_create, execute_kv_append,
    execute_kv_del, execute_kv_put, execute_namespace_create, execute_transfer,
};
use crate::validation::validate_transaction;

/// Result of executing a transaction
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Transaction hash
    pub tx_hash: Hash,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Events generated during execution
    pub events: Vec<ExecutionEvent>,
}

/// Events emitted during transaction execution
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    Transfer {
        from: seloria_core::PublicKey,
        to: seloria_core::PublicKey,
        amount: u64,
    },
    AgentRegistered {
        agent_pubkey: seloria_core::PublicKey,
    },
    ClaimCreated {
        claim_id: Hash,
        claim_type: String,
        creator: seloria_core::PublicKey,
        stake: u64,
    },
    AttestationAdded {
        claim_id: Hash,
        attester: seloria_core::PublicKey,
        vote: seloria_core::Vote,
        stake: u64,
    },
    ClaimFinalized {
        claim_id: Hash,
        status: seloria_core::ClaimStatus,
        yes_stake: u64,
        no_stake: u64,
    },
    NamespaceCreated {
        ns_id: Hash,
        owner: seloria_core::PublicKey,
    },
    KvUpdated {
        ns_id: Hash,
        key: String,
    },
    KvDeleted {
        ns_id: Hash,
        key: String,
    },
    AppRegistered {
        app_id: Hash,
    },
}

/// Transaction executor
pub struct Executor {
    /// Current timestamp for time-based checks
    current_time: u64,
    /// Current block height
    current_height: u64,
}

impl Executor {
    pub fn new(current_time: u64, current_height: u64) -> Self {
        Executor {
            current_time,
            current_height,
        }
    }

    /// Execute a single transaction
    pub fn execute_transaction<S: Storage>(
        &self,
        tx: &Transaction,
        state: &mut ChainState<S>,
    ) -> ExecutionResult {
        let tx_hash = match tx.hash() {
            Ok(h) => h,
            Err(e) => {
                return ExecutionResult {
                    tx_hash: Hash::ZERO,
                    success: false,
                    error: Some(format!("Failed to hash transaction: {}", e)),
                    events: vec![],
                }
            }
        };

        debug!("Executing transaction {}", tx_hash);

        // Validate transaction
        let validation = validate_transaction(tx, state, self.current_time);
        if !validation.is_valid {
            let error_msg = validation
                .error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "Unknown validation error".to_string());
            error!("Transaction {} validation failed: {}", tx_hash, error_msg);
            return ExecutionResult {
                tx_hash,
                success: false,
                error: Some(error_msg),
                events: vec![],
            };
        }

        // Deduct fee
        if let Err(e) = state.deduct_fee(&tx.sender_pubkey, tx.fee) {
            return ExecutionResult {
                tx_hash,
                success: false,
                error: Some(format!("Failed to deduct fee: {}", e)),
                events: vec![],
            };
        }
        state.distribute_fee_to_validators(tx.fee);

        // Execute operations
        let mut events = Vec::new();

        for op in &tx.ops {
            match self.execute_op(op, &tx.sender_pubkey, state, &mut events) {
                Ok(()) => {}
                Err(e) => {
                    error!("Operation execution failed: {}", e);
                    // Rollback would happen here in a real implementation
                    return ExecutionResult {
                        tx_hash,
                        success: false,
                        error: Some(e.to_string()),
                        events,
                    };
                }
            }
        }

        // Increment nonce
        state.increment_nonce(&tx.sender_pubkey);

        info!("Transaction {} executed successfully", tx_hash);

        ExecutionResult {
            tx_hash,
            success: true,
            error: None,
            events,
        }
    }

    /// Execute a single operation
    fn execute_op<S: Storage>(
        &self,
        op: &Op,
        sender: &seloria_core::PublicKey,
        state: &mut ChainState<S>,
        events: &mut Vec<ExecutionEvent>,
    ) -> Result<(), VmError> {
        match op {
            Op::AgentCertRegister { cert } => {
                execute_agent_cert_register(state, sender, cert, self.current_time)?;
                events.push(ExecutionEvent::AgentRegistered {
                    agent_pubkey: cert.cert.agent_pubkey,
                });
            }

            Op::Transfer { to, amount } => {
                execute_transfer(state, sender, to, *amount)?;
                events.push(ExecutionEvent::Transfer {
                    from: *sender,
                    to: *to,
                    amount: *amount,
                });
            }

            Op::ClaimCreate {
                claim_type,
                payload_hash,
                stake,
            } => {
                let claim_id = execute_claim_create(
                    state,
                    sender,
                    claim_type,
                    payload_hash,
                    *stake,
                    self.current_height,
                )?;
                events.push(ExecutionEvent::ClaimCreated {
                    claim_id,
                    claim_type: claim_type.clone(),
                    creator: *sender,
                    stake: *stake,
                });
            }

            Op::Attest {
                claim_id,
                vote,
                stake,
            } => {
                let finalized =
                    execute_attest(state, sender, claim_id, *vote, *stake, self.current_height)?;

                events.push(ExecutionEvent::AttestationAdded {
                    claim_id: *claim_id,
                    attester: *sender,
                    vote: *vote,
                    stake: *stake,
                });

                if finalized {
                    let claim = state.get_claim(claim_id).unwrap();
                    events.push(ExecutionEvent::ClaimFinalized {
                        claim_id: *claim_id,
                        status: claim.status,
                        yes_stake: claim.yes_stake,
                        no_stake: claim.no_stake,
                    });
                }
            }

            Op::AppRegister { meta } => {
                // Check app doesn't already exist
                if state.get_app(&meta.app_id).is_some() {
                    return Err(VmError::AppExists(meta.app_id.to_hex()));
                }

                if meta.publisher != *sender {
                    return Err(VmError::InvalidOperation(
                        "App publisher must match sender".to_string(),
                    ));
                }

                let app = AppMeta {
                    app_id: meta.app_id,
                    version: meta.version.clone(),
                    publisher: meta.publisher,
                    metadata_hash: meta.metadata_hash,
                    namespaces: meta.namespaces.clone(),
                    schemas: meta.schemas.clone(),
                    recipes: meta.recipes.clone(),
                    registered_at: self.current_height,
                };

                state.register_app(app);
                events.push(ExecutionEvent::AppRegistered {
                    app_id: meta.app_id,
                });
            }

            Op::KvPut { ns_id, key, value } => {
                execute_kv_put(state, sender, ns_id, key, value.clone())?;
                events.push(ExecutionEvent::KvUpdated {
                    ns_id: *ns_id,
                    key: key.clone(),
                });
            }

            Op::KvDel { ns_id, key } => {
                execute_kv_del(state, sender, ns_id, key)?;
                events.push(ExecutionEvent::KvDeleted {
                    ns_id: *ns_id,
                    key: key.clone(),
                });
            }

            Op::KvAppend { ns_id, key, value } => {
                execute_kv_append(state, sender, ns_id, key, value.clone())?;
                events.push(ExecutionEvent::KvUpdated {
                    ns_id: *ns_id,
                    key: key.clone(),
                });
            }

            Op::NamespaceCreate {
                ns_id,
                policy,
                allowlist,
                min_write_stake,
            } => {
                execute_namespace_create(
                    state,
                    sender,
                    ns_id,
                    policy.clone(),
                    allowlist.clone(),
                    *min_write_stake,
                )?;
                events.push(ExecutionEvent::NamespaceCreated {
                    ns_id: *ns_id,
                    owner: *sender,
                });
            }
        }

        Ok(())
    }

    /// Execute multiple transactions
    pub fn execute_transactions<S: Storage>(
        &self,
        txs: &[Transaction],
        state: &mut ChainState<S>,
    ) -> Vec<ExecutionResult> {
        txs.iter()
            .map(|tx| self.execute_transaction(tx, state))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{
        AgentCertificate, Capability, GenesisConfig, Hash, KeyPair, SignedAgentCertificate, Vote,
        hash_blake3,
    };
    use seloria_state::MemoryStorage;

    fn setup_state_and_agent() -> (ChainState<MemoryStorage>, KeyPair, KeyPair) {
        let mut state = ChainState::new(MemoryStorage::new());
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();

        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![(agent.public, 1_000_000)],
            trusted_issuers: vec![issuer.public],
            validators: vec![],
        };
        state.init_genesis(&config).unwrap();

        // Register agent
        let cert = AgentCertificate::new(
            hash_blake3(issuer.public.as_bytes()),
            agent.public,
            0,
            1_000_000,
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
    fn test_execute_transfer() {
        let (mut state, _, agent) = setup_state_and_agent();
        let receiver = KeyPair::generate();
        let executor = Executor::new(100, 1);

        let tx = Transaction::new_signed(
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

        let result = executor.execute_transaction(&tx, &mut state);

        assert!(result.success);
        assert!(result.events.iter().any(|e| matches!(e, ExecutionEvent::Transfer { .. })));
        assert_eq!(state.get_balance(&receiver.public), 1000);
    }

    #[test]
    fn test_execute_claim_lifecycle() {
        let (mut state, issuer, agent) = setup_state_and_agent();

        // Register another agent for attestation
        let attester = KeyPair::generate();
        state.get_or_create_account(&attester.public).balance = 100_000;

        let attester_cert = AgentCertificate::new(
            hash_blake3(issuer.public.as_bytes()),
            attester.public,
            0,
            1_000_000,
            vec![Capability::TxSubmit, Capability::Attest],
            Hash::ZERO,
        );
        let signed_attester_cert =
            SignedAgentCertificate::new(attester_cert, &issuer.secret).unwrap();
        state.register_agent(signed_attester_cert);

        let executor = Executor::new(100, 1);

        // Create claim
        let payload_hash = hash_blake3(b"test claim payload");
        let tx_create = Transaction::new_signed(
            agent.public,
            1,
            100,
            vec![Op::ClaimCreate {
                claim_type: "test".to_string(),
                payload_hash,
                stake: 1000,
            }],
            &agent.secret,
        )
        .unwrap();

        let result = executor.execute_transaction(&tx_create, &mut state);
        assert!(result.success);

        let claim_id = match &result.events[0] {
            ExecutionEvent::ClaimCreated { claim_id, .. } => *claim_id,
            _ => panic!("Expected ClaimCreated event"),
        };

        // Attest with enough to finalize
        let executor2 = Executor::new(100, 2);
        let tx_attest = Transaction::new_signed(
            attester.public,
            1,
            100,
            vec![Op::Attest {
                claim_id,
                vote: Vote::Yes,
                stake: 1000,
            }],
            &attester.secret,
        )
        .unwrap();

        let result = executor2.execute_transaction(&tx_attest, &mut state);
        assert!(result.success);
        assert!(result
            .events
            .iter()
            .any(|e| matches!(e, ExecutionEvent::ClaimFinalized { .. })));
    }

    #[test]
    fn test_execute_invalid_nonce() {
        let (mut state, _, agent) = setup_state_and_agent();
        let receiver = KeyPair::generate();
        let executor = Executor::new(100, 1);

        let tx = Transaction::new_signed(
            agent.public,
            5, // Wrong nonce
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 1000,
            }],
            &agent.secret,
        )
        .unwrap();

        let result = executor.execute_transaction(&tx, &mut state);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid nonce"));
    }
}
