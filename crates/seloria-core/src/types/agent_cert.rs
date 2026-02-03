use serde::{Deserialize, Serialize};

use crate::crypto::{hash_blake3, verify, Hash, PublicKey, SecretKey, Sig, sign};
use crate::error::CoreError;
use crate::serialize;

/// Capabilities that an agent can be granted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Can submit transactions
    TxSubmit,
    /// Can create claims
    Claim,
    /// Can attest to claims
    Attest,
    /// Can write to KV store
    KvWrite,
}

/// An agent certificate issued by a trusted issuer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCertificate {
    /// Certificate version
    pub version: u8,
    /// Hash identifying the issuer
    pub issuer_id: Hash,
    /// Agent's public key
    pub agent_pubkey: PublicKey,
    /// Unique agent ID (derived from certificate contents)
    pub agent_id: Hash,
    /// Unix timestamp when issued
    pub issued_at: u64,
    /// Unix timestamp when expires
    pub expires_at: u64,
    /// List of capabilities granted
    pub capabilities: Vec<Capability>,
    /// Hash of additional metadata (stored off-chain)
    pub metadata_hash: Hash,
}

impl AgentCertificate {
    pub const CURRENT_VERSION: u8 = 1;

    /// Create a new agent certificate
    pub fn new(
        issuer_id: Hash,
        agent_pubkey: PublicKey,
        issued_at: u64,
        expires_at: u64,
        capabilities: Vec<Capability>,
        metadata_hash: Hash,
    ) -> Self {
        let mut cert = AgentCertificate {
            version: Self::CURRENT_VERSION,
            issuer_id,
            agent_pubkey,
            agent_id: Hash::ZERO, // Will be computed
            issued_at,
            expires_at,
            capabilities,
            metadata_hash,
        };
        cert.agent_id = cert.compute_agent_id();
        cert
    }

    /// Compute the agent ID from certificate contents
    pub fn compute_agent_id(&self) -> Hash {
        // Create a deterministic representation for hashing
        let mut data = Vec::new();
        data.push(self.version);
        data.extend_from_slice(self.issuer_id.as_bytes());
        data.extend_from_slice(self.agent_pubkey.as_bytes());
        data.extend_from_slice(&self.issued_at.to_le_bytes());
        data.extend_from_slice(&self.expires_at.to_le_bytes());
        for cap in &self.capabilities {
            data.push(*cap as u8);
        }
        data.extend_from_slice(self.metadata_hash.as_bytes());
        hash_blake3(&data)
    }

    /// Check if the certificate is expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.expires_at
    }

    /// Check if the certificate has a specific capability
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Get bytes for signing
    pub fn signing_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serialize::to_bytes(self)
    }
}

/// A signed agent certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAgentCertificate {
    pub cert: AgentCertificate,
    pub issuer_signature: Sig,
}

impl SignedAgentCertificate {
    /// Create and sign a new agent certificate
    pub fn new(cert: AgentCertificate, issuer_secret: &SecretKey) -> Result<Self, CoreError> {
        let signing_bytes = cert.signing_bytes()?;
        let signature = sign(issuer_secret, &signing_bytes);
        Ok(SignedAgentCertificate {
            cert,
            issuer_signature: signature,
        })
    }

    /// Verify the issuer signature
    pub fn verify_signature(&self, issuer_pubkey: &PublicKey) -> Result<(), CoreError> {
        let signing_bytes = self.cert.signing_bytes()?;
        verify(issuer_pubkey, &signing_bytes, &self.issuer_signature)
    }

    /// Check if expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.cert.is_expired(current_time)
    }

    /// Check capability
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.cert.has_capability(cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    fn create_test_cert(
        issuer: &KeyPair,
        agent: &KeyPair,
    ) -> SignedAgentCertificate {
        let issuer_id = hash_blake3(issuer.public.as_bytes());
        let cert = AgentCertificate::new(
            issuer_id,
            agent.public,
            1000,
            2000,
            vec![Capability::TxSubmit, Capability::Claim],
            Hash::ZERO,
        );
        SignedAgentCertificate::new(cert, &issuer.secret).unwrap()
    }

    #[test]
    fn test_certificate_creation() {
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();
        let signed = create_test_cert(&issuer, &agent);

        assert_eq!(signed.cert.version, AgentCertificate::CURRENT_VERSION);
        assert_eq!(signed.cert.agent_pubkey, agent.public);
        assert!(signed.has_capability(Capability::TxSubmit));
        assert!(!signed.has_capability(Capability::KvWrite));
    }

    #[test]
    fn test_certificate_signature_verification() {
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();
        let signed = create_test_cert(&issuer, &agent);

        assert!(signed.verify_signature(&issuer.public).is_ok());
    }

    #[test]
    fn test_certificate_wrong_issuer() {
        let issuer = KeyPair::generate();
        let wrong_issuer = KeyPair::generate();
        let agent = KeyPair::generate();
        let signed = create_test_cert(&issuer, &agent);

        assert!(signed.verify_signature(&wrong_issuer.public).is_err());
    }

    #[test]
    fn test_certificate_expiration() {
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();
        let signed = create_test_cert(&issuer, &agent);

        assert!(!signed.is_expired(1500));
        assert!(signed.is_expired(2000));
        assert!(signed.is_expired(2500));
    }

    #[test]
    fn test_agent_id_deterministic() {
        let issuer = KeyPair::generate();
        let agent = KeyPair::generate();

        let issuer_id = hash_blake3(issuer.public.as_bytes());
        let cert1 = AgentCertificate::new(
            issuer_id,
            agent.public,
            1000,
            2000,
            vec![Capability::TxSubmit],
            Hash::ZERO,
        );
        let cert2 = AgentCertificate::new(
            issuer_id,
            agent.public,
            1000,
            2000,
            vec![Capability::TxSubmit],
            Hash::ZERO,
        );

        assert_eq!(cert1.agent_id, cert2.agent_id);
    }
}
