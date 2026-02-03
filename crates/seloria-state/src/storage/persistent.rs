use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use seloria_core::serialize;

use super::Storage;
use crate::error::StateError;

/// File-backed storage using a single snapshot file.
#[derive(Debug, Clone)]
pub struct FileStorage {
    path: PathBuf,
    data: BTreeMap<Vec<u8>, Vec<u8>>,
    pending_writes: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
}

impl FileStorage {
    pub fn new<P: Into<PathBuf>>(path: P) -> Result<Self, StateError> {
        let path = path.into();
        let data = if path.exists() {
            let bytes = fs::read(&path).map_err(|e| StateError::Storage(e.to_string()))?;
            if bytes.is_empty() {
                BTreeMap::new()
            } else {
                serialize::from_bytes(&bytes)
                    .map_err(|e| StateError::Serialization(e.to_string()))?
            }
        } else {
            BTreeMap::new()
        };

        Ok(FileStorage {
            path,
            data,
            pending_writes: BTreeMap::new(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn flush_to_disk(&self) -> Result<(), StateError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| StateError::Storage(e.to_string()))?;
        }

        let bytes =
            serialize::to_bytes(&self.data).map_err(|e| StateError::Serialization(e.to_string()))?;
        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, &bytes).map_err(|e| StateError::Storage(e.to_string()))?;
        fs::rename(&tmp_path, &self.path).map_err(|e| StateError::Storage(e.to_string()))?;
        Ok(())
    }
}

impl Storage for FileStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if let Some(pending) = self.pending_writes.get(key) {
            return pending.clone();
        }
        self.data.get(key).cloned()
    }

    fn put(&mut self, key: &[u8], value: &[u8]) {
        self.pending_writes
            .insert(key.to_vec(), Some(value.to_vec()));
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
        self.flush_to_disk()?;
        Ok(())
    }

    fn rollback(&mut self) {
        self.pending_writes.clear();
    }

    fn keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>> {
        let mut keys = Vec::new();

        for key in self.data.keys() {
            if key.starts_with(prefix) {
                if let Some(pending) = self.pending_writes.get(key) {
                    if pending.is_some() {
                        keys.push(key.clone());
                    }
                } else {
                    keys.push(key.clone());
                }
            }
        }

        for (key, value) in &self.pending_writes {
            if key.starts_with(prefix) && value.is_some() && !self.data.contains_key(key) {
                keys.push(key.clone());
            }
        }

        keys
    }
}
