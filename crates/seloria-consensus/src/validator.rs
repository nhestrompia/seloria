use seloria_core::{sign, Block, PublicKey, SecretKey, Sig};
use seloria_state::{ChainState, Storage};
use tracing::{debug, info};

use crate::block_builder::BlockBuilder;
use crate::error::ConsensusError;

/// A validator node that signs blocks
pub struct Validator {
    /// Validator's keypair
    pub public_key: PublicKey,
    secret_key: SecretKey,
    /// Block builder for validation
    block_builder: BlockBuilder,
}

impl Validator {
    /// Create a new validator
    pub fn new(public_key: PublicKey, secret_key: SecretKey, block_builder: BlockBuilder) -> Self {
        Validator {
            public_key,
            secret_key,
            block_builder,
        }
    }

    /// Validate a proposed block and sign it if valid
    pub fn validate_and_sign<S: Storage + Clone>(
        &self,
        block: &Block,
        state: &ChainState<S>,
    ) -> Result<Sig, ConsensusError> {
        info!(
            "Validating block {} at height {}",
            block.hash()?,
            block.header.height
        );

        // Basic validation
        self.block_builder.validate_block(block, state)?;

        // Re-execute and verify on a clone of state
        self.block_builder.verify_execution(block, state)?;

        // Sign the block
        let block_hash = block.hash()?;
        let signature = sign(&self.secret_key, block_hash.as_bytes());

        debug!(
            "Validator {} signed block {}",
            self.public_key, block_hash
        );

        Ok(signature)
    }

    /// Check if this validator is the leader for a given height
    pub fn is_leader(&self, height: u64, validators: &[PublicKey]) -> bool {
        if validators.is_empty() {
            return false;
        }
        let leader_index = (height as usize) % validators.len();
        validators.get(leader_index) == Some(&self.public_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_builder::BlockBuilderConfig;
    use seloria_core::{
        verify, AgentCertificate, Capability, GenesisConfig, Hash, KeyPair,
        SignedAgentCertificate, hash_blake3,
    };
    use seloria_mempool::{Mempool, MempoolConfig};
    use seloria_state::MemoryStorage;

    #[tokio::test]
    async fn test_validator_sign() {
        let mut state = ChainState::new(MemoryStorage::new());
        let issuer = KeyPair::generate();
        let validator_kp = KeyPair::generate();

        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![],
            trusted_issuers: vec![issuer.public],
            validators: vec![validator_kp.public],
        };
        state.init_genesis(&config).unwrap();

        let block_builder = BlockBuilder::new(BlockBuilderConfig {
            chain_id: 1,
            ..Default::default()
        });

        let mempool = Mempool::new(MempoolConfig::default());
        let block = block_builder
            .build_block(&state, &mempool, validator_kp.public, 1000)
            .await
            .unwrap();

        // Reset height for validation (simulating a fresh validator)
        state.height = 0;
        state.head_block = Some(config.create_genesis_block());

        let validator = Validator::new(validator_kp.public, validator_kp.secret.clone(), block_builder);
        let signature = validator.validate_and_sign(&block, &state).unwrap();

        // Verify signature
        let block_hash = block.hash().unwrap();
        verify(&validator_kp.public, block_hash.as_bytes(), &signature).unwrap();
    }

    #[test]
    fn test_leader_rotation() {
        let validators: Vec<_> = (0..4).map(|_| KeyPair::generate()).collect();
        let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();

        let block_builder = BlockBuilder::new(BlockBuilderConfig::default());

        for (i, kp) in validators.iter().enumerate() {
            let validator = Validator::new(
                kp.public,
                kp.secret.clone(),
                BlockBuilder::new(BlockBuilderConfig::default()),
            );

            // At height i, validator i should be leader
            assert!(validator.is_leader(i as u64, &validator_pubkeys));
            // At height i+4, validator i should be leader again
            assert!(validator.is_leader((i + 4) as u64, &validator_pubkeys));
            // At height i+1, validator i should not be leader
            assert!(!validator.is_leader((i + 1) as u64 % 4, &validator_pubkeys));
        }
    }
}
