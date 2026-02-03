use seloria_core::{merkle_root, Block, BlockHeader, Hash, PublicKey, Transaction};
use seloria_mempool::Mempool;
use seloria_state::{ChainState, Storage};
use seloria_vm::{ExecutionResult, Executor};
use tracing::{debug, info};

use crate::error::ConsensusError;

/// Configuration for block building
#[derive(Debug, Clone)]
pub struct BlockBuilderConfig {
    /// Maximum transactions per block
    pub max_transactions: usize,
    /// Chain ID
    pub chain_id: u64,
}

impl Default for BlockBuilderConfig {
    fn default() -> Self {
        BlockBuilderConfig {
            max_transactions: 1000,
            chain_id: 1,
        }
    }
}

/// Block builder for creating new blocks
pub struct BlockBuilder {
    config: BlockBuilderConfig,
}

impl BlockBuilder {
    pub fn new(config: BlockBuilderConfig) -> Self {
        BlockBuilder { config }
    }

    /// Build a new block from mempool transactions
    pub async fn build_block<S: Storage + Clone>(
        &self,
        state: &ChainState<S>,
        mempool: &Mempool,
        proposer: PublicKey,
        timestamp: u64,
    ) -> Result<Block, ConsensusError> {
        let current_height = state.current_height();
        let next_height = current_height + 1;

        info!("Building block at height {}", next_height);

        // Get previous block hash
        let prev_hash = if let Some(ref head) = state.head_block {
            head.hash()?
        } else {
            Hash::ZERO
        };

        // Get transactions from mempool
        let pending_txs = mempool.get_transactions(self.config.max_transactions).await;
        debug!("Got {} transactions from mempool", pending_txs.len());

        // Execute transactions on a working copy and collect successful ones
        let mut working_state = state.clone();
        let executor = Executor::new(timestamp, next_height);
        let mut successful_txs = Vec::new();

        for tx in pending_txs {
            let result = executor.execute_transaction(&tx, &mut working_state);
            if result.success {
                successful_txs.push(tx);
            } else {
                debug!(
                    "Transaction {} failed: {:?}",
                    result.tx_hash,
                    result.error
                );
            }
        }

        info!(
            "Executed {} successful transactions for block {}",
            successful_txs.len(),
            next_height
        );

        // Compute merkle roots
        let tx_hashes: Result<Vec<Hash>, _> =
            successful_txs.iter().map(|tx| tx.hash()).collect();
        let tx_root = merkle_root(&tx_hashes?);

        let state_root = working_state.compute_state_root()?;

        // Create block header
        let header = BlockHeader {
            chain_id: self.config.chain_id,
            height: next_height,
            prev_hash,
            timestamp,
            tx_root,
            state_root,
            proposer_pubkey: proposer,
        };

        let block = Block::new(header, successful_txs);

        debug!("Built block {} with hash {}", next_height, block.hash()?);

        Ok(block)
    }

    /// Validate a proposed block
    pub fn validate_block<S: Storage>(
        &self,
        block: &Block,
        state: &ChainState<S>,
    ) -> Result<(), ConsensusError> {
        let expected_height = state.current_height() + 1;

        // Check height
        if block.header.height != expected_height {
            return Err(ConsensusError::HeightMismatch {
                expected: expected_height,
                got: block.header.height,
            });
        }

        // Check previous hash
        let expected_prev_hash = if let Some(ref head) = state.head_block {
            head.hash()?
        } else {
            Hash::ZERO
        };

        if block.header.prev_hash != expected_prev_hash {
            return Err(ConsensusError::PrevHashMismatch);
        }

        // Verify transaction root
        if !block.verify_tx_root()? {
            return Err(ConsensusError::InvalidBlock(
                "Transaction root mismatch".to_string(),
            ));
        }

        // Check chain ID
        if block.header.chain_id != self.config.chain_id {
            return Err(ConsensusError::InvalidBlock(format!(
                "Chain ID mismatch: expected {}, got {}",
                self.config.chain_id, block.header.chain_id
            )));
        }

        Ok(())
    }

    /// Re-execute block transactions and verify state root
    pub fn verify_execution<S: Storage + Clone>(
        &self,
        block: &Block,
        state: &ChainState<S>,
    ) -> Result<(), ConsensusError> {
        let executor = Executor::new(block.header.timestamp, block.header.height);
        let mut state_copy = state.clone();

        // Execute all transactions
        for tx in &block.txs {
            let result = executor.execute_transaction(tx, &mut state_copy);
            if !result.success {
                return Err(ConsensusError::ExecutionFailed(
                    result.error.unwrap_or_else(|| "Unknown error".to_string()),
                ));
            }
        }

        // Verify state root
        let computed_root = state_copy.compute_state_root()?;
        if computed_root != block.header.state_root {
            return Err(ConsensusError::InvalidStateRoot);
        }

        Ok(())
    }

    /// Apply a block by re-executing transactions and updating state.
    /// Returns execution results for event emission.
    pub fn apply_block<S: Storage + Clone>(
        &self,
        state: &mut ChainState<S>,
        block: &Block,
    ) -> Result<Vec<ExecutionResult>, ConsensusError> {
        // Validate header and tx root against current state
        self.validate_block(block, state)?;

        let mut working_state = state.clone();
        let executor = Executor::new(block.header.timestamp, block.header.height);
        let mut results = Vec::new();

        for tx in &block.txs {
            let result = executor.execute_transaction(tx, &mut working_state);
            if !result.success {
                return Err(ConsensusError::ExecutionFailed(
                    result.error.unwrap_or_else(|| "Unknown error".to_string()),
                ));
            }
            results.push(result);
        }

        let computed_root = working_state.compute_state_root()?;
        if computed_root != block.header.state_root {
            return Err(ConsensusError::InvalidStateRoot);
        }

        working_state.apply_block(block.clone())?;
        *state = working_state;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{
        AgentCertificate, Capability, GenesisConfig, KeyPair, Op, SignedAgentCertificate,
        hash_blake3,
    };
    use seloria_mempool::MempoolConfig;
    use seloria_state::MemoryStorage;

    async fn setup_test_env() -> (
        ChainState<MemoryStorage>,
        Mempool,
        KeyPair,
        KeyPair,
        BlockBuilder,
    ) {
        let mut state = ChainState::new(MemoryStorage::new());
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();
        let proposer = KeyPair::generate();

        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![(agent.public, 1_000_000)],
            trusted_issuers: vec![issuer.public],
            validators: vec![proposer.public],
        };
        state.init_genesis(&config).unwrap();

        // Register agent
        let cert = AgentCertificate::new(
            hash_blake3(issuer.public.as_bytes()),
            agent.public,
            0,
            1_000_000,
            vec![Capability::TxSubmit],
            Hash::ZERO,
        );
        let signed_cert = SignedAgentCertificate::new(cert, &issuer.secret).unwrap();
        state.register_agent(signed_cert);

        let mempool = Mempool::new(MempoolConfig::default());
        let builder = BlockBuilder::new(BlockBuilderConfig {
            chain_id: 1,
            ..Default::default()
        });

        (state, mempool, agent, proposer, builder)
    }

    #[tokio::test]
    async fn test_build_empty_block() {
        let (mut state, mempool, _, proposer, builder) = setup_test_env().await;

        let block = builder
            .build_block(&state, &mempool, proposer.public, 1000)
            .await
            .unwrap();

        assert_eq!(block.header.height, 1);
        assert!(block.txs.is_empty());
    }

    #[tokio::test]
    async fn test_build_block_with_transactions() {
        let (mut state, mempool, agent, proposer, builder) = setup_test_env().await;

        // Add transaction to mempool
        let receiver = KeyPair::generate();
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

        mempool.add(tx).await.unwrap();

        let block = builder
            .build_block(&state, &mempool, proposer.public, 1000)
            .await
            .unwrap();

        assert_eq!(block.header.height, 1);
        assert_eq!(block.txs.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_block() {
        let (mut state, mempool, _, proposer, builder) = setup_test_env().await;

        let block = builder
            .build_block(&state, &mempool, proposer.public, 1000)
            .await
            .unwrap();

        // Reset state to validate fresh
        let mut fresh_state = ChainState::new(MemoryStorage::new());
        let issuer = KeyPair::generate();
        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![],
            trusted_issuers: vec![issuer.public],
            validators: vec![proposer.public],
        };
        fresh_state.init_genesis(&config).unwrap();

        builder.validate_block(&block, &fresh_state).unwrap();
    }
}
