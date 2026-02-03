use serde::{Deserialize, Serialize};

use crate::crypto::{Hash, PublicKey};

/// Metadata for a registered application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMeta {
    /// Unique app ID (derived from content)
    pub app_id: Hash,
    /// Version string
    pub version: String,
    /// Publisher's public key
    pub publisher: PublicKey,
    /// Hash of the app metadata/specification
    pub metadata_hash: Hash,
    /// Namespace IDs owned/used by this app
    pub namespaces: Vec<Hash>,
    /// Schema hashes associated with this app
    pub schemas: Vec<Hash>,
    /// Recipe hashes for client conventions
    pub recipes: Vec<Hash>,
    /// Block height when registered
    pub registered_at: u64,
}

impl AppMeta {
    /// Create new app metadata
    pub fn new(
        app_id: Hash,
        version: String,
        publisher: PublicKey,
        metadata_hash: Hash,
        namespaces: Vec<Hash>,
        schemas: Vec<Hash>,
        recipes: Vec<Hash>,
        registered_at: u64,
    ) -> Self {
        AppMeta {
            app_id,
            version,
            publisher,
            metadata_hash,
            namespaces,
            schemas,
            recipes,
            registered_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{hash_blake3, KeyPair};

    #[test]
    fn test_app_meta_creation() {
        let owner = KeyPair::generate();
        let app_id = hash_blake3(b"app");
        let schema_hash = hash_blake3(b"schema");

        let app = AppMeta::new(
            app_id,
            "1.0.0".to_string(),
            owner.public,
            schema_hash,
            vec![hash_blake3(b"ns")],
            vec![hash_blake3(b"schema")],
            vec![hash_blake3(b"recipe")],
            1,
        );

        assert_eq!(app.version, "1.0.0");
        assert_eq!(app.publisher, owner.public);
    }
}
