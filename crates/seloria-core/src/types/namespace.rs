use serde::{Deserialize, Serialize};

use crate::crypto::{Hash, PublicKey};

/// Policy for namespace access control
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamespacePolicy {
    /// Only the owner can write
    OwnerOnly,
    /// Only addresses on the allowlist can write
    Allowlist,
    /// Anyone with sufficient stake can write
    StakeGated,
}

/// Metadata for a namespace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceMeta {
    /// Unique namespace ID
    pub ns_id: Hash,
    /// Owner's public key
    pub owner: PublicKey,
    /// Access control policy
    pub policy: NamespacePolicy,
    /// Allowlist for Allowlist policy
    pub allowlist: Vec<PublicKey>,
    /// Minimum stake required for StakeGated policy
    pub min_write_stake: u64,
}

impl NamespaceMeta {
    /// Create a new namespace with OwnerOnly policy
    pub fn new_owner_only(ns_id: Hash, owner: PublicKey) -> Self {
        NamespaceMeta {
            ns_id,
            owner,
            policy: NamespacePolicy::OwnerOnly,
            allowlist: Vec::new(),
            min_write_stake: 0,
        }
    }

    /// Create a new namespace with Allowlist policy
    pub fn new_allowlist(ns_id: Hash, owner: PublicKey, allowlist: Vec<PublicKey>) -> Self {
        NamespaceMeta {
            ns_id,
            owner,
            policy: NamespacePolicy::Allowlist,
            allowlist,
            min_write_stake: 0,
        }
    }

    /// Create a new namespace with StakeGated policy
    pub fn new_stake_gated(ns_id: Hash, owner: PublicKey, min_stake: u64) -> Self {
        NamespaceMeta {
            ns_id,
            owner,
            policy: NamespacePolicy::StakeGated,
            allowlist: Vec::new(),
            min_write_stake: min_stake,
        }
    }

    /// Check if a public key can write to this namespace
    pub fn can_write(&self, writer: &PublicKey, writer_stake: u64) -> bool {
        match &self.policy {
            NamespacePolicy::OwnerOnly => *writer == self.owner,
            NamespacePolicy::Allowlist => {
                *writer == self.owner || self.allowlist.contains(writer)
            }
            NamespacePolicy::StakeGated => writer_stake >= self.min_write_stake,
        }
    }

    /// Add a public key to the allowlist (only owner can do this)
    pub fn add_to_allowlist(&mut self, pubkey: PublicKey) {
        if !self.allowlist.contains(&pubkey) {
            self.allowlist.push(pubkey);
        }
    }

    /// Remove a public key from the allowlist
    pub fn remove_from_allowlist(&mut self, pubkey: &PublicKey) {
        self.allowlist.retain(|pk| pk != pubkey);
    }
}

/// Value stored in the KV store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvValue {
    /// Content encoding (e.g., "json", "cbor", "raw")
    pub codec: String,
    /// The actual data
    pub data: KvData,
}

impl KvValue {
    /// Create an inline value
    pub fn inline(codec: &str, data: Vec<u8>) -> Self {
        KvValue {
            codec: codec.to_string(),
            data: KvData::Inline(data),
        }
    }

    /// Create a reference value
    pub fn reference(codec: &str, hash: Hash, uri: Option<String>) -> Self {
        KvValue {
            codec: codec.to_string(),
            data: KvData::Reference { hash, uri },
        }
    }
}

/// KV data can be inline or a reference to external storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvData {
    /// Data stored directly in the chain
    Inline(Vec<u8>),
    /// Reference to externally stored data
    Reference {
        /// Hash of the data for verification
        hash: Hash,
        /// Optional URI for retrieval
        uri: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{hash_blake3, KeyPair};

    #[test]
    fn test_owner_only_policy() {
        let owner = KeyPair::generate();
        let other = KeyPair::generate();
        let ns_id = hash_blake3(b"namespace");

        let ns = NamespaceMeta::new_owner_only(ns_id, owner.public);

        assert!(ns.can_write(&owner.public, 0));
        assert!(!ns.can_write(&other.public, 0));
    }

    #[test]
    fn test_allowlist_policy() {
        let owner = KeyPair::generate();
        let allowed = KeyPair::generate();
        let other = KeyPair::generate();
        let ns_id = hash_blake3(b"namespace");

        let ns = NamespaceMeta::new_allowlist(ns_id, owner.public, vec![allowed.public]);

        assert!(ns.can_write(&owner.public, 0));
        assert!(ns.can_write(&allowed.public, 0));
        assert!(!ns.can_write(&other.public, 0));
    }

    #[test]
    fn test_stake_gated_policy() {
        let owner = KeyPair::generate();
        let rich = KeyPair::generate();
        let poor = KeyPair::generate();
        let ns_id = hash_blake3(b"namespace");

        let ns = NamespaceMeta::new_stake_gated(ns_id, owner.public, 1000);

        assert!(ns.can_write(&rich.public, 1000));
        assert!(ns.can_write(&rich.public, 2000));
        assert!(!ns.can_write(&poor.public, 500));
    }

    #[test]
    fn test_kv_value_inline() {
        let value = KvValue::inline("json", b"{\"key\": \"value\"}".to_vec());
        assert_eq!(value.codec, "json");
        match value.data {
            KvData::Inline(data) => assert_eq!(data, b"{\"key\": \"value\"}"),
            _ => panic!("Expected inline data"),
        }
    }

    #[test]
    fn test_kv_value_reference() {
        let hash = hash_blake3(b"data");
        let value = KvValue::reference("raw", hash, Some("ipfs://...".to_string()));
        assert_eq!(value.codec, "raw");
        match value.data {
            KvData::Reference { hash: h, uri } => {
                assert_eq!(h, hash);
                assert_eq!(uri, Some("ipfs://...".to_string()));
            }
            _ => panic!("Expected reference data"),
        }
    }
}
