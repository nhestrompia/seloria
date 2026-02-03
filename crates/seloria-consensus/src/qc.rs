use seloria_core::{verify, Hash, PublicKey, QuorumCertificate, Sig, ValidatorSignature};
use std::collections::HashSet;
use tracing::debug;

use crate::error::ConsensusError;

/// Quorum certificate builder/collector
pub struct QcBuilder {
    block_hash: Hash,
    signatures: Vec<ValidatorSignature>,
    validators: HashSet<PublicKey>,
    threshold: usize,
}

impl QcBuilder {
    /// Create a new QC builder for a block
    pub fn new(block_hash: Hash, validators: &[PublicKey], threshold: usize) -> Self {
        QcBuilder {
            block_hash,
            signatures: Vec::new(),
            validators: validators.iter().copied().collect(),
            threshold,
        }
    }

    /// Add a validator signature
    pub fn add_signature(
        &mut self,
        validator: PublicKey,
        signature: Sig,
    ) -> Result<bool, ConsensusError> {
        // Check validator is in our set
        if !self.validators.contains(&validator) {
            return Err(ConsensusError::ValidatorNotFound(validator.to_hex()));
        }

        // Verify signature
        verify(&validator, self.block_hash.as_bytes(), &signature)?;

        // Check for duplicate
        if self
            .signatures
            .iter()
            .any(|vs| vs.validator_pubkey == validator)
        {
            debug!("Duplicate signature from {}", validator);
            return Ok(self.has_quorum());
        }

        self.signatures.push(ValidatorSignature {
            validator_pubkey: validator,
            signature,
        });

        debug!(
            "Added signature from {}, total: {}/{}",
            validator,
            self.signatures.len(),
            self.threshold
        );

        Ok(self.has_quorum())
    }

    /// Check if we have enough signatures
    pub fn has_quorum(&self) -> bool {
        self.signatures.len() >= self.threshold
    }

    /// Get current signature count
    pub fn signature_count(&self) -> usize {
        self.signatures.len()
    }

    /// Build the quorum certificate (only if we have quorum)
    pub fn build(self) -> Result<QuorumCertificate, ConsensusError> {
        if !self.has_quorum() {
            return Err(ConsensusError::InsufficientSignatures {
                have: self.signatures.len(),
                need: self.threshold,
            });
        }

        Ok(QuorumCertificate {
            block_hash: self.block_hash,
            signatures: self.signatures,
        })
    }
}

/// Verify a quorum certificate
pub fn verify_qc(
    qc: &QuorumCertificate,
    validators: &[PublicKey],
    threshold: usize,
) -> Result<(), ConsensusError> {
    let validator_set: HashSet<_> = validators.iter().collect();

    // Check we have enough signatures
    if qc.signatures.len() < threshold {
        return Err(ConsensusError::InsufficientSignatures {
            have: qc.signatures.len(),
            need: threshold,
        });
    }

    // Verify each signature
    for vs in &qc.signatures {
        if !validator_set.contains(&vs.validator_pubkey) {
            return Err(ConsensusError::ValidatorNotFound(
                vs.validator_pubkey.to_hex(),
            ));
        }

        verify(&vs.validator_pubkey, qc.block_hash.as_bytes(), &vs.signature)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{hash_blake3, sign, KeyPair};

    fn create_validators(n: usize) -> Vec<KeyPair> {
        (0..n).map(|_| KeyPair::generate()).collect()
    }

    #[test]
    fn test_qc_builder_basic() {
        let validators = create_validators(4);
        let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();
        let block_hash = hash_blake3(b"test block");

        let mut builder = QcBuilder::new(block_hash, &validator_pubkeys, 3);

        // Add 3 signatures
        for validator in &validators[..3] {
            let sig = sign(&validator.secret, block_hash.as_bytes());
            let has_quorum = builder.add_signature(validator.public, sig).unwrap();
            if validators.iter().position(|v| v.public == validator.public).unwrap() < 2 {
                assert!(!has_quorum);
            } else {
                assert!(has_quorum);
            }
        }

        let qc = builder.build().unwrap();
        assert_eq!(qc.signatures.len(), 3);
    }

    #[test]
    fn test_qc_builder_invalid_validator() {
        let validators = create_validators(4);
        let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();
        let block_hash = hash_blake3(b"test block");

        let mut builder = QcBuilder::new(block_hash, &validator_pubkeys, 3);

        let outsider = KeyPair::generate();
        let sig = sign(&outsider.secret, block_hash.as_bytes());
        let result = builder.add_signature(outsider.public, sig);

        assert!(matches!(result, Err(ConsensusError::ValidatorNotFound(_))));
    }

    #[test]
    fn test_qc_builder_invalid_signature() {
        let validators = create_validators(4);
        let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();
        let block_hash = hash_blake3(b"test block");

        let mut builder = QcBuilder::new(block_hash, &validator_pubkeys, 3);

        // Sign wrong message
        let wrong_sig = sign(&validators[0].secret, b"wrong message");
        let result = builder.add_signature(validators[0].public, wrong_sig);

        assert!(matches!(result, Err(ConsensusError::Core(_))));
    }

    #[test]
    fn test_verify_qc() {
        let validators = create_validators(4);
        let validator_pubkeys: Vec<_> = validators.iter().map(|v| v.public).collect();
        let block_hash = hash_blake3(b"test block");

        let qc = QuorumCertificate {
            block_hash,
            signatures: validators[..3]
                .iter()
                .map(|v| ValidatorSignature {
                    validator_pubkey: v.public,
                    signature: sign(&v.secret, block_hash.as_bytes()),
                })
                .collect(),
        };

        verify_qc(&qc, &validator_pubkeys, 3).unwrap();
    }
}
