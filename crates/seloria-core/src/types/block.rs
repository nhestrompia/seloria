use serde::{Deserialize, Serialize};

use crate::crypto::{hash_blake3, merkle_root, sign, verify, Hash, PublicKey, SecretKey, Sig};
use crate::error::CoreError;
use crate::serialize;
use crate::types::transaction::Transaction;

/// Block header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Chain identifier
    pub chain_id: u64,
    /// Block height (0 for genesis)
    pub height: u64,
    /// Hash of the previous block (zeros for genesis)
    pub prev_hash: Hash,
    /// Unix timestamp
    pub timestamp: u64,
    /// Merkle root of transactions
    pub tx_root: Hash,
    /// State root after applying transactions
    pub state_root: Hash,
    /// Proposer's public key
    pub proposer_pubkey: PublicKey,
}

impl BlockHeader {
    /// Compute the hash of this header
    pub fn hash(&self) -> Result<Hash, CoreError> {
        let bytes = serialize::to_bytes(self)?;
        Ok(hash_blake3(&bytes))
    }
}

/// A signature from a validator on a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    pub validator_pubkey: PublicKey,
    pub signature: Sig,
}

/// Quorum certificate proving validator consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumCertificate {
    /// Hash of the block being certified
    pub block_hash: Hash,
    /// Validator signatures
    pub signatures: Vec<ValidatorSignature>,
}

impl QuorumCertificate {
    /// Create a new empty QC
    pub fn new(block_hash: Hash) -> Self {
        QuorumCertificate {
            block_hash,
            signatures: Vec::new(),
        }
    }

    /// Add a validator signature
    pub fn add_signature(&mut self, validator_pubkey: PublicKey, signature: Sig) {
        self.signatures.push(ValidatorSignature {
            validator_pubkey,
            signature,
        });
    }

    /// Verify all signatures in the QC
    pub fn verify_signatures(&self) -> Result<(), CoreError> {
        for vs in &self.signatures {
            verify(&vs.validator_pubkey, self.block_hash.as_bytes(), &vs.signature)?;
        }
        Ok(())
    }

    /// Check if quorum is reached (requires threshold signatures)
    pub fn has_quorum(&self, threshold: usize) -> bool {
        self.signatures.len() >= threshold
    }

    /// Get the number of signatures
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }
}

/// A complete block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Transaction>,
    pub qc: Option<QuorumCertificate>,
}

impl Block {
    /// Create a new block
    pub fn new(header: BlockHeader, txs: Vec<Transaction>) -> Self {
        Block {
            header,
            txs,
            qc: None,
        }
    }

    /// Compute the block hash (hash of header)
    pub fn hash(&self) -> Result<Hash, CoreError> {
        self.header.hash()
    }

    /// Compute the merkle root of transactions
    pub fn compute_tx_root(&self) -> Result<Hash, CoreError> {
        let tx_hashes: Result<Vec<Hash>, _> = self.txs.iter().map(|tx| tx.hash()).collect();
        Ok(merkle_root(&tx_hashes?))
    }

    /// Verify that tx_root matches transactions
    pub fn verify_tx_root(&self) -> Result<bool, CoreError> {
        let computed = self.compute_tx_root()?;
        Ok(computed == self.header.tx_root)
    }

    /// Sign the block as a validator
    pub fn sign_as_validator(&self, secret_key: &SecretKey) -> Result<Sig, CoreError> {
        let block_hash = self.hash()?;
        Ok(sign(secret_key, block_hash.as_bytes()))
    }

    /// Add a validator signature to the QC
    pub fn add_validator_signature(
        &mut self,
        validator_pubkey: PublicKey,
        signature: Sig,
    ) -> Result<(), CoreError> {
        let block_hash = self.hash()?;

        if self.qc.is_none() {
            self.qc = Some(QuorumCertificate::new(block_hash));
        }

        if let Some(ref mut qc) = self.qc {
            qc.add_signature(validator_pubkey, signature);
        }

        Ok(())
    }
}

/// Genesis block configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain_id: u64,
    pub timestamp: u64,
    pub initial_balances: Vec<(PublicKey, u64)>,
    pub trusted_issuers: Vec<PublicKey>,
    pub validators: Vec<PublicKey>,
}

impl GenesisConfig {
    /// Create a genesis block from this config
    pub fn create_genesis_block(&self) -> Block {
        let header = BlockHeader {
            chain_id: self.chain_id,
            height: 0,
            prev_hash: Hash::ZERO,
            timestamp: self.timestamp,
            tx_root: Hash::ZERO, // No transactions in genesis
            state_root: Hash::ZERO, // Will be computed after applying initial state
            proposer_pubkey: PublicKey::default(),
        };

        Block::new(header, Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::types::transaction::Op;

    fn create_test_block() -> Block {
        let proposer = KeyPair::generate();
        let sender = KeyPair::generate();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            sender.public,
            1,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 500,
            }],
            &sender.secret,
        )
        .unwrap();

        let tx_root = merkle_root(&[tx.hash().unwrap()]);

        let header = BlockHeader {
            chain_id: 1,
            height: 1,
            prev_hash: Hash::ZERO,
            timestamp: 1000,
            tx_root,
            state_root: Hash::ZERO,
            proposer_pubkey: proposer.public,
        };

        Block::new(header, vec![tx])
    }

    #[test]
    fn test_block_hash_deterministic() {
        let block = create_test_block();
        let hash1 = block.hash().unwrap();
        let hash2 = block.hash().unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_block_tx_root_verification() {
        let block = create_test_block();
        assert!(block.verify_tx_root().unwrap());
    }

    #[test]
    fn test_validator_signature() {
        let mut block = create_test_block();
        let validator = KeyPair::generate();

        let sig = block.sign_as_validator(&validator.secret).unwrap();
        block.add_validator_signature(validator.public, sig).unwrap();

        assert!(block.qc.is_some());
        let qc = block.qc.as_ref().unwrap();
        assert_eq!(qc.signature_count(), 1);
        assert!(qc.verify_signatures().is_ok());
    }

    #[test]
    fn test_quorum_threshold() {
        let mut block = create_test_block();

        for _ in 0..3 {
            let validator = KeyPair::generate();
            let sig = block.sign_as_validator(&validator.secret).unwrap();
            block.add_validator_signature(validator.public, sig).unwrap();
        }

        let qc = block.qc.as_ref().unwrap();
        assert!(qc.has_quorum(3));
        assert!(!qc.has_quorum(4));
    }

    #[test]
    fn test_genesis_config() {
        let issuer = KeyPair::generate();
        let validator = KeyPair::generate();
        let account = KeyPair::generate();

        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![(account.public, 1_000_000)],
            trusted_issuers: vec![issuer.public],
            validators: vec![validator.public],
        };

        let genesis = config.create_genesis_block();
        assert_eq!(genesis.header.height, 0);
        assert_eq!(genesis.header.prev_hash, Hash::ZERO);
        assert!(genesis.txs.is_empty());
    }
}
