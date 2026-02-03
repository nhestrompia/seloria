pub mod memory;
pub mod persistent;

use crate::error::StateError;

/// Storage trait for chain state persistence
pub trait Storage: Send + Sync {
    /// Get a value by key
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Put a key-value pair
    fn put(&mut self, key: &[u8], value: &[u8]);

    /// Delete a key
    fn delete(&mut self, key: &[u8]);

    /// Commit pending changes
    fn commit(&mut self) -> Result<(), StateError>;

    /// Rollback pending changes
    fn rollback(&mut self);

    /// Check if a key exists
    fn exists(&self, key: &[u8]) -> bool {
        self.get(key).is_some()
    }

    /// Get all keys with a given prefix
    fn keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>>;
}

pub use memory::MemoryStorage;
pub use persistent::FileStorage;
