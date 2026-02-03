use std::sync::Arc;
use std::time::Duration;

use seloria_core::{Block, Hash, PublicKey, SecretKey};
use seloria_mempool::Mempool;
use seloria_state::{ChainState, Storage};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::block_builder::{BlockBuilder, BlockBuilderConfig};
use crate::events::BlockEventSink;
use crate::error::ConsensusError;
use crate::qc::QcBuilder;
use crate::net::{CommitRequest, ProposeRequest, ProposeResponse};

#[derive(Debug, Clone)]
pub struct ValidatorEndpoint {
    pub pubkey: PublicKey,
    pub address: String,
}

impl ValidatorEndpoint {
    pub fn new(pubkey: PublicKey, address: String) -> Self {
        ValidatorEndpoint { pubkey, address }
    }
}

/// Configuration for the proposer
#[derive(Debug, Clone)]
pub struct ProposerConfig {
    /// Round time in milliseconds
    pub round_time_ms: u64,
    /// Number of validators
    pub num_validators: usize,
    /// Quorum threshold
    pub threshold: usize,
    /// Chain ID
    pub chain_id: u64,
    /// Max transactions per block
    pub max_block_txs: usize,
}

impl Default for ProposerConfig {
    fn default() -> Self {
        ProposerConfig {
            round_time_ms: 2000,
            num_validators: 4,
            threshold: 3,
            chain_id: 1,
            max_block_txs: 1000,
        }
    }
}

/// Block proposer that builds and broadcasts blocks
pub struct Proposer<S: Storage> {
    config: ProposerConfig,
    public_key: PublicKey,
    secret_key: SecretKey,
    block_builder: BlockBuilder,
    state: Arc<RwLock<ChainState<S>>>,
    mempool: Arc<Mempool>,
    validators: Vec<PublicKey>,
    validator_endpoints: Vec<ValidatorEndpoint>,
    event_sink: Option<Arc<dyn BlockEventSink>>,
}

impl<S: Storage + Send + Sync + Clone + 'static> Proposer<S> {
    /// Create a new proposer
    pub fn new(
        config: ProposerConfig,
        public_key: PublicKey,
        secret_key: SecretKey,
        state: Arc<RwLock<ChainState<S>>>,
        mempool: Arc<Mempool>,
        validators: Vec<PublicKey>,
    ) -> Self {
        let block_builder = BlockBuilder::new(BlockBuilderConfig {
            chain_id: config.chain_id,
            max_transactions: config.max_block_txs,
        });

        Proposer {
            config,
            public_key,
            secret_key,
            block_builder,
            state,
            mempool,
            validators,
            validator_endpoints: Vec::new(),
            event_sink: None,
        }
    }

    pub fn set_validator_endpoints(&mut self, endpoints: Vec<ValidatorEndpoint>) {
        self.validator_endpoints = endpoints;
    }

    pub fn set_event_sink(&mut self, sink: Arc<dyn BlockEventSink>) {
        self.event_sink = Some(sink);
    }

    /// Check if we are the leader for the current height
    pub async fn is_current_leader(&self) -> bool {
        let state = self.state.read().await;
        let next_height = state.current_height() + 1;
        self.is_leader_for_height(next_height)
    }

    /// Check if we are the leader for a given height
    fn is_leader_for_height(&self, height: u64) -> bool {
        if self.validators.is_empty() {
            return false;
        }
        let leader_index = (height as usize) % self.validators.len();
        self.validators.get(leader_index) == Some(&self.public_key)
    }

    /// Get the current timestamp
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Propose a new block
    pub async fn propose_block(&self) -> Result<Block, ConsensusError> {
        // Check if we're the leader
        if !self.is_current_leader().await {
            return Err(ConsensusError::NotLeader);
        }

        let timestamp = Self::current_timestamp();
        let state = self.state.read().await;

        let block = self
            .block_builder
            .build_block(&*state, &self.mempool, self.public_key, timestamp)
            .await?;

        info!(
            "Proposed block {} at height {}",
            block.hash()?,
            block.header.height
        );

        Ok(block)
    }

    /// Finalize a block with collected signatures
    pub async fn finalize_block(&self, mut block: Block) -> Result<Block, ConsensusError> {
        let block_hash = block.hash()?;

        // Create QC builder
        let mut qc_builder =
            QcBuilder::new(block_hash, &self.validators, self.config.threshold);

        // Add our own signature
        let our_sig = seloria_core::sign(&self.secret_key, block_hash.as_bytes());
        qc_builder.add_signature(self.public_key, our_sig)?;

        // Collect signatures from other validators if configured
        if !self.validator_endpoints.is_empty() {
            self.collect_signatures(&block, &mut qc_builder).await;
        }

        if !qc_builder.has_quorum() {
            return Err(ConsensusError::InsufficientSignatures {
                have: qc_builder.signature_count(),
                need: self.config.threshold,
            });
        }

        block.qc = Some(qc_builder.build()?);

        Ok(block)
    }

    /// Apply a finalized block to state
    pub async fn apply_block(&self, block: Block) -> Result<(), ConsensusError>
    where
        S: Clone,
    {
        let block_hash = block.hash()?;
        let height = block.header.height;

        // Apply block to state
        let mut state = self.state.write().await;
        let results = self.block_builder.apply_block(&mut *state, &block)?;
        state.persist_state()?;
        drop(state);

        // Remove committed transactions from mempool
        let tx_hashes: Vec<Hash> = block
            .txs
            .iter()
            .filter_map(|tx| tx.hash().ok())
            .collect();
        self.mempool.remove_committed(&tx_hashes).await;

        info!("Applied block {} at height {}", block_hash, height);

        if let Some(sink) = &self.event_sink {
            sink.on_block_committed(&block, &results);
        }

        Ok(())
    }

    /// Run the proposer loop (single node mode)
    pub async fn run_single_node(self: Arc<Self>) {
        let round_duration = Duration::from_millis(self.config.round_time_ms);
        let mut round_interval = interval(round_duration);

        info!(
            "Starting proposer loop with round time {}ms",
            self.config.round_time_ms
        );

        loop {
            round_interval.tick().await;

            if !self.is_current_leader().await {
                continue;
            }

            // Propose block
            match self.propose_block().await {
                Ok(block) => {
                    // In single-node mode, immediately finalize and apply
                    match self.finalize_block(block).await {
                        Ok(finalized_block) => {
                            let commit_block = finalized_block.clone();
                            if let Err(e) = self.apply_block(finalized_block).await {
                                error!("Failed to apply block: {}", e);
                            } else if !self.validator_endpoints.is_empty() {
                                self.broadcast_commit(&commit_block).await;
                            }
                        }
                        Err(e) => {
                            error!("Failed to finalize block: {}", e);
                        }
                    }
                }
                Err(e) => {
                    if !matches!(e, ConsensusError::NotLeader) {
                        warn!("Failed to propose block: {}", e);
                    }
                }
            }
        }
    }

    async fn collect_signatures(
        &self,
        block: &Block,
        qc_builder: &mut QcBuilder,
    ) {
        let client = reqwest::Client::new();

        for endpoint in &self.validator_endpoints {
            if endpoint.pubkey == self.public_key {
                continue;
            }

            let url = format!(
                "{}/consensus/propose",
                endpoint.address.trim_end_matches('/')
            );
            let request = ProposeRequest { block: block.clone() };

            let response = client.post(&url).json(&request).send().await;
            let response = match response {
                Ok(resp) => resp,
                Err(e) => {
                    warn!("Failed to reach validator {}: {}", endpoint.pubkey, e);
                    continue;
                }
            };

            if !response.status().is_success() {
                warn!(
                    "Validator {} rejected propose: {}",
                    endpoint.pubkey,
                    response.status()
                );
                continue;
            }

            let body: ProposeResponse = match response.json().await {
                Ok(body) => body,
                Err(e) => {
                    warn!("Invalid propose response from {}: {}", endpoint.pubkey, e);
                    continue;
                }
            };

            if let Err(e) = qc_builder.add_signature(body.validator_pubkey, body.signature) {
                warn!("Invalid signature from {}: {}", body.validator_pubkey, e);
                continue;
            }

            if qc_builder.has_quorum() {
                break;
            }
        }
    }

    pub async fn broadcast_commit(&self, block: &Block) {
        if self.validator_endpoints.is_empty() {
            return;
        }

        let client = reqwest::Client::new();
        let request = CommitRequest { block: block.clone() };

        for endpoint in &self.validator_endpoints {
            if endpoint.pubkey == self.public_key {
                continue;
            }

            let url = format!(
                "{}/consensus/commit",
                endpoint.address.trim_end_matches('/')
            );

            let response = client.post(&url).json(&request).send().await;
            if let Err(e) = response {
                warn!("Failed to broadcast commit to {}: {}", endpoint.pubkey, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{GenesisConfig, KeyPair};
    use seloria_mempool::MempoolConfig;
    use seloria_state::MemoryStorage;

    #[tokio::test]
    async fn test_proposer_creation() {
        let state = Arc::new(RwLock::new(ChainState::new(MemoryStorage::new())));
        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
        let validator = KeyPair::generate();

        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![],
            trusted_issuers: vec![],
            validators: vec![validator.public],
        };
        state.write().await.init_genesis(&config).unwrap();

        let proposer = Proposer::new(
            ProposerConfig::default(),
            validator.public,
            validator.secret,
            state,
            mempool,
            vec![validator.public],
        );

        assert!(proposer.is_current_leader().await);
    }

    #[tokio::test]
    async fn test_propose_and_apply() {
        let state = Arc::new(RwLock::new(ChainState::new(MemoryStorage::new())));
        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
        let validator = KeyPair::generate();

        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![],
            trusted_issuers: vec![],
            validators: vec![validator.public],
        };
        state.write().await.init_genesis(&config).unwrap();

        let proposer_config = ProposerConfig {
            threshold: 1, // Single validator
            ..Default::default()
        };

        let proposer = Proposer::new(
            proposer_config,
            validator.public,
            validator.secret,
            state.clone(),
            mempool,
            vec![validator.public],
        );

        // Propose block
        let block = proposer.propose_block().await.unwrap();
        assert_eq!(block.header.height, 1);

        // Finalize and apply
        let finalized = proposer.finalize_block(block).await.unwrap();
        proposer.apply_block(finalized).await.unwrap();

        // Check height updated
        let state_read = state.read().await;
        assert_eq!(state_read.current_height(), 1);
    }
}
