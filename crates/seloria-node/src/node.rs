use std::sync::Arc;

use anyhow::Result;
use seloria_consensus::{BlockEventSink, Proposer, ProposerConfig, ValidatorEndpoint};
use seloria_core::{Block, KeyPair, SecretKey};
use seloria_mempool::{Mempool, MempoolConfig};
use seloria_rpc::{RpcConfig, RpcServer, WsEvent};
use seloria_rpc::ws::EventBroadcaster;
use seloria_state::{ChainState, FileStorage};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

use crate::config::NodeConfig;

/// The Seloria node
pub struct Node {
    config: NodeConfig,
    state: Arc<RwLock<ChainState<FileStorage>>>,
    mempool: Arc<Mempool>,
    broadcaster: Arc<EventBroadcaster>,
    validator_keypair: Option<Arc<Mutex<KeyPair>>>,
    validator_endpoints: Vec<ValidatorEndpoint>,
    issuer_keypair: Option<Arc<Mutex<KeyPair>>>,
}

struct RpcEventSink {
    broadcaster: Arc<EventBroadcaster>,
}

impl BlockEventSink for RpcEventSink {
    fn on_block_committed(
        &self,
        block: &Block,
        results: &[seloria_vm::ExecutionResult],
    ) {
        if let Ok(hash) = block.hash() {
            self.broadcaster.broadcast(WsEvent::block_committed(
                block.header.height,
                hash,
                block.txs.len(),
                block.header.timestamp,
            ));
        }

        for (tx, result) in block.txs.iter().zip(results.iter()) {
            self.broadcaster.broadcast(WsEvent::tx_applied(
                result.tx_hash,
                tx.sender_pubkey,
                result.success,
            ));

            for event in &result.events {
                if let Some(ws_event) = WsEvent::from_execution_event(event) {
                    self.broadcaster.broadcast(ws_event);
                }
            }
        }
    }
}

impl Node {
    /// Create a new node from configuration
    pub fn new(config: NodeConfig) -> Result<Self> {
        // Parse validator key if present
        let validator_keypair = if let Some(ref key_hex) = config.validator_key {
            let secret = SecretKey::from_hex(key_hex)?;
            let public = secret.public_key();
            Some(Arc::new(Mutex::new(KeyPair { secret, public })))
        } else {
            None
        };

        let issuer_keypair = if let Some(ref key_hex) = config.issuer_key {
            let secret = SecretKey::from_hex(key_hex)?;
            let public = secret.public_key();
            Some(Arc::new(Mutex::new(KeyPair { secret, public })))
        } else {
            None
        };

        // Create state
        let storage_path = config.data_dir.join("state.bin");
        let storage = FileStorage::new(storage_path)?;
        let state = Arc::new(RwLock::new(ChainState::new(storage)));

        // Create mempool
        let mempool_config = MempoolConfig {
            max_size: config.mempool_max_size,
            max_per_sender: config.mempool_max_per_sender,
            ..Default::default()
        };
        let mempool = Arc::new(Mempool::new(mempool_config));

        // Create event broadcaster
        let broadcaster = Arc::new(EventBroadcaster::default());

        let validator_endpoints = config
            .validator_endpoints
            .iter()
            .map(|endpoint| {
                let pubkey = seloria_core::PublicKey::from_hex(&endpoint.pubkey)?;
                Ok(ValidatorEndpoint::new(pubkey, endpoint.address.clone()))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Node {
            config,
            state,
            mempool,
            broadcaster,
            validator_keypair,
            validator_endpoints,
            issuer_keypair,
        })
    }

    /// Initialize genesis state
    pub async fn init_genesis(&self) -> Result<()> {
        let genesis_config = self.config.to_genesis_config()?;

        let mut state = self.state.write().await;
        state.init_genesis(&genesis_config)?;

        info!("Genesis state initialized");
        Ok(())
    }

    /// Run the node
    pub async fn run(self) -> Result<()> {
        info!("Starting Seloria node");

        // Load persisted state if available
        {
            let mut state = self.state.write().await;
            state.load_from_storage()?;
        }

        // Initialize genesis if needed
        {
            let state = self.state.read().await;
            if state.head_block.is_none() {
                drop(state);
                self.init_genesis().await?;
            }
        }

        // Get validators
        let validators = {
            let state = self.state.read().await;
            state.validators.clone()
        };

        // Start RPC server
        let rpc_config = RpcConfig {
            http_addr: self.config.rpc_addr,
            enable_ws: self.config.enable_ws,
        };

        let rpc_server = RpcServer::new(
            rpc_config,
            Arc::clone(&self.state),
            Arc::clone(&self.mempool),
            Arc::clone(&self.broadcaster),
            self.validator_keypair.clone(),
            self.issuer_keypair.clone(),
            Some(self.config.data_dir.join("state.bin")),
        );

        let rpc_router = rpc_server.router();
        let rpc_addr = self.config.rpc_addr;

        // Start proposer if we're a validator
        let proposer_handle = if let Some(ref keypair) = self.validator_keypair {
            let keypair = keypair.lock().await.clone();
            let proposer_config = ProposerConfig {
                round_time_ms: self.config.round_time_ms,
                num_validators: validators.len(),
                threshold: (validators.len() * 2 / 3) + 1,
                chain_id: self.config.chain_id,
                max_block_txs: self.config.max_block_txs,
            };

            let mut proposer = Proposer::new(
                proposer_config,
                keypair.public,
                keypair.secret.clone(),
                Arc::clone(&self.state),
                Arc::clone(&self.mempool),
                validators,
            );

            if !self.validator_endpoints.is_empty() {
                proposer.set_validator_endpoints(self.validator_endpoints.clone());
            }

            proposer.set_event_sink(Arc::new(RpcEventSink {
                broadcaster: Arc::clone(&self.broadcaster),
            }));

            let proposer = Arc::new(proposer);

            info!("Starting as validator: {}", keypair.public);

            Some(tokio::spawn(async move {
                proposer.run_single_node().await;
            }))
        } else {
            info!("Starting as non-validator node");
            None
        };

        // Run RPC server (this will block)
        info!("RPC server listening on {}", rpc_addr);
        let listener = tokio::net::TcpListener::bind(rpc_addr).await?;

        if let Err(e) = axum::serve(listener, rpc_router).await {
            error!("RPC server error: {}", e);
        }

        // Wait for proposer if running
        if let Some(handle) = proposer_handle {
            handle.await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::generate_sample_config;

    #[tokio::test]
    async fn test_node_creation() {
        let config = generate_sample_config();
        let node = Node::new(config).unwrap();

        assert!(node.validator_keypair.is_some());
    }

    #[tokio::test]
    async fn test_genesis_init() {
        let config = generate_sample_config();
        let node = Node::new(config).unwrap();

        node.init_genesis().await.unwrap();

        let state = node.state.read().await;
        assert!(state.head_block.is_some());
    }
}
