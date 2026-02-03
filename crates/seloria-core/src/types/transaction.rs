use serde::{Deserialize, Serialize};

use crate::crypto::{hash_blake3, sign, verify, Hash, PublicKey, SecretKey, Sig};
use crate::error::CoreError;
use crate::serialize;
use crate::types::agent_cert::SignedAgentCertificate;
use crate::types::app::AppMeta;
use crate::types::claim::Vote;
use crate::types::namespace::KvValue;

/// Operations that can be included in a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    /// Register an agent certificate
    AgentCertRegister {
        cert: SignedAgentCertificate,
    },
    /// Transfer tokens to another account
    Transfer {
        to: PublicKey,
        amount: u64,
    },
    /// Create a new claim
    ClaimCreate {
        claim_type: String,
        payload_hash: Hash,
        stake: u64,
    },
    /// Attest to an existing claim
    Attest {
        claim_id: Hash,
        vote: Vote,
        stake: u64,
    },
    /// Register an application
    AppRegister {
        meta: AppMeta,
    },
    /// Put a key-value pair in a namespace
    KvPut {
        ns_id: Hash,
        key: String,
        value: KvValue,
    },
    /// Delete a key from a namespace
    KvDel {
        ns_id: Hash,
        key: String,
    },
    /// Append to an existing key's value
    KvAppend {
        ns_id: Hash,
        key: String,
        value: KvValue,
    },
    /// Create a new namespace
    NamespaceCreate {
        ns_id: Hash,
        policy: crate::types::namespace::NamespacePolicy,
        allowlist: Vec<PublicKey>,
        min_write_stake: u64,
    },
}

/// A transaction containing one or more operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Sender's public key
    pub sender_pubkey: PublicKey,
    /// Transaction nonce (must equal account nonce + 1)
    pub nonce: u64,
    /// Fee to pay for transaction processing
    pub fee: u64,
    /// List of operations to execute
    pub ops: Vec<Op>,
    /// Signature over the transaction (excluding this field)
    pub signature: Sig,
}

/// Transaction data for signing (excludes signature field)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionSigningData {
    sender_pubkey: PublicKey,
    nonce: u64,
    fee: u64,
    ops: Vec<Op>,
}

impl Transaction {
    /// Create a new unsigned transaction
    pub fn new(sender_pubkey: PublicKey, nonce: u64, fee: u64, ops: Vec<Op>) -> Self {
        Transaction {
            sender_pubkey,
            nonce,
            fee,
            ops,
            signature: Sig::default(),
        }
    }

    /// Get the data to be signed
    fn signing_data(&self) -> TransactionSigningData {
        TransactionSigningData {
            sender_pubkey: self.sender_pubkey,
            nonce: self.nonce,
            fee: self.fee,
            ops: self.ops.clone(),
        }
    }

    /// Get bytes for signing
    pub fn signing_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serialize::to_bytes(&self.signing_data())
    }

    /// Sign the transaction
    pub fn sign(&mut self, secret_key: &SecretKey) -> Result<(), CoreError> {
        let bytes = self.signing_bytes()?;
        self.signature = sign(secret_key, &bytes);
        Ok(())
    }

    /// Create a signed transaction
    pub fn new_signed(
        sender_pubkey: PublicKey,
        nonce: u64,
        fee: u64,
        ops: Vec<Op>,
        secret_key: &SecretKey,
    ) -> Result<Self, CoreError> {
        let mut tx = Self::new(sender_pubkey, nonce, fee, ops);
        tx.sign(secret_key)?;
        Ok(tx)
    }

    /// Verify the transaction signature
    pub fn verify_signature(&self) -> Result<(), CoreError> {
        let bytes = self.signing_bytes()?;
        verify(&self.sender_pubkey, &bytes, &self.signature)
    }

    /// Compute the transaction hash
    pub fn hash(&self) -> Result<Hash, CoreError> {
        let bytes = serialize::to_bytes(self)?;
        Ok(hash_blake3(&bytes))
    }

    /// Estimate the total cost of this transaction (fee + locked stakes)
    pub fn estimated_cost(&self) -> u64 {
        let mut cost = self.fee;

        for op in &self.ops {
            match op {
                Op::Transfer { amount, .. } => cost += amount,
                Op::ClaimCreate { stake, .. } => cost += stake,
                Op::Attest { stake, .. } => cost += stake,
                _ => {}
            }
        }

        cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;

    #[test]
    fn test_transaction_signing() {
        let sender = KeyPair::generate();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            sender.public,
            1,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 1000,
            }],
            &sender.secret,
        )
        .unwrap();

        assert!(tx.verify_signature().is_ok());
    }

    #[test]
    fn test_transaction_wrong_signature() {
        let sender = KeyPair::generate();
        let wrong_signer = KeyPair::generate();
        let receiver = KeyPair::generate();

        let tx = Transaction::new_signed(
            sender.public, // Sender pubkey
            1,
            100,
            vec![Op::Transfer {
                to: receiver.public,
                amount: 1000,
            }],
            &wrong_signer.secret, // Wrong signer!
        )
        .unwrap();

        assert!(tx.verify_signature().is_err());
    }

    #[test]
    fn test_transaction_hash_deterministic() {
        let sender = KeyPair::generate();

        let tx = Transaction::new_signed(
            sender.public,
            1,
            100,
            vec![Op::Transfer {
                to: sender.public,
                amount: 500,
            }],
            &sender.secret,
        )
        .unwrap();

        let hash1 = tx.hash().unwrap();
        let hash2 = tx.hash().unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_estimated_cost() {
        let sender = KeyPair::generate();
        let receiver = KeyPair::generate();

        let tx = Transaction::new(
            sender.public,
            1,
            100, // fee
            vec![
                Op::Transfer {
                    to: receiver.public,
                    amount: 1000,
                },
                Op::ClaimCreate {
                    claim_type: "test".to_string(),
                    payload_hash: Hash::ZERO,
                    stake: 500,
                },
            ],
        );

        assert_eq!(tx.estimated_cost(), 100 + 1000 + 500);
    }
}
