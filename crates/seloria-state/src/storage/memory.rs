use std::collections::BTreeMap;

use super::Storage;
use crate::error::StateError;

/// In-memory storage implementation using BTreeMap
#[derive(Debug, Clone, Default)]
pub struct MemoryStorage {
    /// Committed data
    data: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Pending writes (not yet committed)
    pending_writes: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            data: BTreeMap::new(),
            pending_writes: BTreeMap::new(),
        }
    }

    /// Get the number of committed keys
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get all committed data (for debugging/testing)
    pub fn all_data(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.data
    }
}

impl Storage for MemoryStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        // Check pending writes first
        if let Some(pending) = self.pending_writes.get(key) {
            return pending.clone();
        }
        // Fall back to committed data
        self.data.get(key).cloned()
    }

    fn put(&mut self, key: &[u8], value: &[u8]) {
        self.pending_writes.insert(key.to_vec(), Some(value.to_vec()));
    }

    fn delete(&mut self, key: &[u8]) {
        self.pending_writes.insert(key.to_vec(), None);
    }

    fn commit(&mut self) -> Result<(), StateError> {
        let pending = std::mem::take(&mut self.pending_writes);
        for (key, value) in pending {
            match value {
                Some(v) => {
                    self.data.insert(key, v);
                }
                None => {
                    self.data.remove(&key);
                }
            }
        }
        Ok(())
    }

    fn rollback(&mut self) {
        self.pending_writes.clear();
    }

    fn keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>> {
        let mut keys = Vec::new();

        // Get from committed data
        for key in self.data.keys() {
            if key.starts_with(prefix) {
                // Check if deleted in pending
                if let Some(pending) = self.pending_writes.get(key) {
                    if pending.is_some() {
                        keys.push(key.clone());
                    }
                } else {
                    keys.push(key.clone());
                }
            }
        }

        // Add new keys from pending writes
        for (key, value) in &self.pending_writes {
            if key.starts_with(prefix) && value.is_some() {
                if !self.data.contains_key(key) {
                    keys.push(key.clone());
                }
            }
        }

        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut storage = MemoryStorage::new();

        storage.put(b"key1", b"value1");
        storage.commit().unwrap();

        assert_eq!(storage.get(b"key1"), Some(b"value1".to_vec()));
        assert!(storage.exists(b"key1"));
        assert!(!storage.exists(b"key2"));
    }

    #[test]
    fn test_pending_writes() {
        let mut storage = MemoryStorage::new();

        storage.put(b"key1", b"value1");
        // Not committed yet, but should still be visible
        assert_eq!(storage.get(b"key1"), Some(b"value1".to_vec()));

        storage.rollback();
        assert_eq!(storage.get(b"key1"), None);
    }

    #[test]
    fn test_delete() {
        let mut storage = MemoryStorage::new();

        storage.put(b"key1", b"value1");
        storage.commit().unwrap();

        storage.delete(b"key1");
        assert_eq!(storage.get(b"key1"), None);

        storage.rollback();
        assert_eq!(storage.get(b"key1"), Some(b"value1".to_vec()));

        storage.delete(b"key1");
        storage.commit().unwrap();
        assert_eq!(storage.get(b"key1"), None);
    }

    #[test]
    fn test_prefix_query() {
        let mut storage = MemoryStorage::new();

        storage.put(b"users:1", b"alice");
        storage.put(b"users:2", b"bob");
        storage.put(b"items:1", b"item");
        storage.commit().unwrap();

        let user_keys = storage.keys_with_prefix(b"users:");
        assert_eq!(user_keys.len(), 2);
        assert!(user_keys.contains(&b"users:1".to_vec()));
        assert!(user_keys.contains(&b"users:2".to_vec()));
    }

    #[test]
    fn test_overwrite() {
        let mut storage = MemoryStorage::new();

        storage.put(b"key", b"value1");
        storage.commit().unwrap();

        storage.put(b"key", b"value2");
        assert_eq!(storage.get(b"key"), Some(b"value2".to_vec()));

        storage.commit().unwrap();
        assert_eq!(storage.get(b"key"), Some(b"value2".to_vec()));
    }
}
