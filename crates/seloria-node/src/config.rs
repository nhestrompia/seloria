use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use seloria_core::{GenesisConfig, KeyPair, PublicKey};
use serde::{Deserialize, Serialize};

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Chain ID
    pub chain_id: u64,

    /// Node data directory
    pub data_dir: PathBuf,

    /// RPC bind address
    pub rpc_addr: SocketAddr,

    /// Enable WebSocket
    pub enable_ws: bool,

    /// Consensus round time in milliseconds
    pub round_time_ms: u64,

    /// Block builder max transactions
    pub max_block_txs: usize,

    /// Mempool max size
    pub mempool_max_size: usize,

    /// Mempool max per sender
    pub mempool_max_per_sender: usize,

    /// Genesis configuration
    pub genesis: GenesisConfigFile,

    /// Validator private key (hex) - only for validator nodes
    pub validator_key: Option<String>,

    /// Issuer private key (hex) - enables /cert/issue endpoint
    pub issuer_key: Option<String>,

    /// Optional validator endpoints for committee mode
    pub validator_endpoints: Vec<ValidatorEndpointConfig>,
}

/// Genesis configuration for file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfigFile {
    pub timestamp: u64,
    pub initial_balances: Vec<BalanceEntry>,
    pub trusted_issuers: Vec<String>,
    pub validators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceEntry {
    pub pubkey: String,
    pub balance: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorEndpointConfig {
    pub pubkey: String,
    pub address: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            chain_id: 1,
            data_dir: PathBuf::from("./seloria-data"),
            rpc_addr: "127.0.0.1:8080".parse().unwrap(),
            enable_ws: true,
            round_time_ms: 2000,
            max_block_txs: 1000,
            mempool_max_size: 10_000,
            mempool_max_per_sender: 100,
            genesis: GenesisConfigFile::default(),
            validator_key: None,
            issuer_key: None,
            validator_endpoints: Vec::new(),
        }
    }
}

impl Default for GenesisConfigFile {
    fn default() -> Self {
        GenesisConfigFile {
            timestamp: 0,
            initial_balances: vec![],
            trusted_issuers: vec![],
            validators: vec![],
        }
    }
}

impl NodeConfig {
    /// Load config from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: NodeConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save config to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert genesis config to core type
    pub fn to_genesis_config(&self) -> Result<GenesisConfig> {
        let initial_balances: Result<Vec<(PublicKey, u64)>, _> = self
            .genesis
            .initial_balances
            .iter()
            .map(|entry| {
                PublicKey::from_hex(&entry.pubkey)
                    .map(|pk| (pk, entry.balance))
                    .map_err(|e| anyhow::anyhow!(e))
            })
            .collect();

        let trusted_issuers: Result<Vec<PublicKey>, _> = self
            .genesis
            .trusted_issuers
            .iter()
            .map(|s| PublicKey::from_hex(s).map_err(|e| anyhow::anyhow!(e)))
            .collect();

        let validators: Result<Vec<PublicKey>, _> = self
            .genesis
            .validators
            .iter()
            .map(|s| PublicKey::from_hex(s).map_err(|e| anyhow::anyhow!(e)))
            .collect();

        Ok(GenesisConfig {
            chain_id: self.chain_id,
            timestamp: self.genesis.timestamp,
            initial_balances: initial_balances?,
            trusted_issuers: trusted_issuers?,
            validators: validators?,
        })
    }
}

/// Generate a sample configuration for testing
pub fn generate_sample_config() -> NodeConfig {
    // Generate keys for testing
    let issuer = KeyPair::generate();
    let validator = KeyPair::generate();
    let user = KeyPair::generate();

    NodeConfig {
        chain_id: 1,
        data_dir: PathBuf::from("./seloria-data"),
        rpc_addr: "127.0.0.1:8080".parse().unwrap(),
        enable_ws: true,
        round_time_ms: 2000,
        max_block_txs: 1000,
        mempool_max_size: 10_000,
        mempool_max_per_sender: 100,
        genesis: GenesisConfigFile {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            initial_balances: vec![BalanceEntry {
                pubkey: user.public.to_hex(),
                balance: 1_000_000_000,
            }],
            trusted_issuers: vec![issuer.public.to_hex()],
            validators: vec![validator.public.to_hex()],
        },
        validator_key: Some(validator.secret.to_hex()),
        issuer_key: Some(issuer.secret.to_hex()),
        validator_endpoints: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.chain_id, 1);
    }

    #[test]
    fn test_sample_config() {
        let config = generate_sample_config();
        assert!(!config.genesis.trusted_issuers.is_empty());
        assert!(!config.genesis.validators.is_empty());
        assert!(config.validator_key.is_some());
    }

    #[test]
    fn test_genesis_conversion() {
        let config = generate_sample_config();
        let genesis = config.to_genesis_config().unwrap();
        assert_eq!(genesis.chain_id, config.chain_id);
        assert_eq!(genesis.validators.len(), 1);
    }
}
