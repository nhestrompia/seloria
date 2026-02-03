use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use seloria_core::{Hash, PublicKey, Transaction};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::ordering::{OrderingMode, TxPriority};

/// Configuration for the mempool
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions in the pool
    pub max_size: usize,
    /// Maximum transactions per sender
    pub max_per_sender: usize,
    /// Transaction expiry time in seconds
    pub expiry_seconds: u64,
    /// Ordering mode
    pub ordering_mode: OrderingMode,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        MempoolConfig {
            max_size: 10_000,
            max_per_sender: 100,
            expiry_seconds: 3600, // 1 hour
            ordering_mode: OrderingMode::FeeRate,
        }
    }
}

/// A pending transaction in the mempool
#[derive(Debug, Clone)]
pub struct PendingTransaction {
    pub tx: Transaction,
    pub hash: Hash,
    pub priority: TxPriority,
    pub added_at: u64,
}

/// The transaction mempool
pub struct Mempool {
    config: MempoolConfig,
    /// Transactions indexed by hash
    by_hash: RwLock<HashMap<Hash, PendingTransaction>>,
    /// Transaction hashes indexed by sender
    by_sender: RwLock<HashMap<PublicKey, HashSet<Hash>>>,
    /// Transaction hashes ordered by priority (for fee-rate ordering)
    by_priority: RwLock<BTreeMap<(TxPriority, Hash), Hash>>,
}

impl Mempool {
    pub fn new(config: MempoolConfig) -> Self {
        Mempool {
            config,
            by_hash: RwLock::new(HashMap::new()),
            by_sender: RwLock::new(HashMap::new()),
            by_priority: RwLock::new(BTreeMap::new()),
        }
    }

    /// Get current timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Add a transaction to the mempool
    pub async fn add(&self, tx: Transaction) -> Result<Hash, MempoolError> {
        let hash = tx.hash().map_err(|_| MempoolError::InvalidTransaction)?;
        let timestamp = Self::current_timestamp();
        let priority = TxPriority::from_transaction(&tx, timestamp);

        let mut by_hash = self.by_hash.write().await;
        let mut by_sender = self.by_sender.write().await;
        let mut by_priority = self.by_priority.write().await;

        // Check if already exists
        if by_hash.contains_key(&hash) {
            return Err(MempoolError::AlreadyExists);
        }

        // Check pool size limit
        if by_hash.len() >= self.config.max_size {
            // Try to evict lowest priority transaction
            if !self.evict_lowest_priority(&mut by_hash, &mut by_sender, &mut by_priority) {
                return Err(MempoolError::PoolFull);
            }
        }

        // Check per-sender limit
        let sender_txs = by_sender.entry(tx.sender_pubkey).or_default();
        if sender_txs.len() >= self.config.max_per_sender {
            return Err(MempoolError::SenderLimitReached);
        }

        // Add transaction
        let pending = PendingTransaction {
            tx: tx.clone(),
            hash,
            priority,
            added_at: timestamp,
        };

        by_hash.insert(hash, pending.clone());
        sender_txs.insert(hash);
        by_priority.insert((priority, hash), hash);

        debug!("Added transaction {} to mempool", hash);

        Ok(hash)
    }

    /// Remove a transaction from the mempool
    pub async fn remove(&self, hash: &Hash) -> Option<Transaction> {
        let mut by_hash = self.by_hash.write().await;
        let mut by_sender = self.by_sender.write().await;
        let mut by_priority = self.by_priority.write().await;

        self.remove_internal(hash, &mut by_hash, &mut by_sender, &mut by_priority)
    }

    fn remove_internal(
        &self,
        hash: &Hash,
        by_hash: &mut HashMap<Hash, PendingTransaction>,
        by_sender: &mut HashMap<PublicKey, HashSet<Hash>>,
        by_priority: &mut BTreeMap<(TxPriority, Hash), Hash>,
    ) -> Option<Transaction> {
        if let Some(pending) = by_hash.remove(hash) {
            if let Some(sender_txs) = by_sender.get_mut(&pending.tx.sender_pubkey) {
                sender_txs.remove(hash);
                if sender_txs.is_empty() {
                    by_sender.remove(&pending.tx.sender_pubkey);
                }
            }
            by_priority.remove(&(pending.priority, *hash));
            debug!("Removed transaction {} from mempool", hash);
            Some(pending.tx)
        } else {
            None
        }
    }

    /// Get a transaction by hash
    pub async fn get(&self, hash: &Hash) -> Option<Transaction> {
        let by_hash = self.by_hash.read().await;
        by_hash.get(hash).map(|p| p.tx.clone())
    }

    /// Check if a transaction exists
    pub async fn contains(&self, hash: &Hash) -> bool {
        let by_hash = self.by_hash.read().await;
        by_hash.contains_key(hash)
    }

    /// Get transactions for block building, ordered by priority
    pub async fn get_transactions(&self, max_count: usize) -> Vec<Transaction> {
        let by_hash = self.by_hash.read().await;
        let by_priority = self.by_priority.read().await;

        match self.config.ordering_mode {
            OrderingMode::FeeRate => {
                // Return highest priority first (reverse iterator)
                by_priority
                    .iter()
                    .rev()
                    .take(max_count)
                    .filter_map(|(_, hash)| by_hash.get(hash).map(|p| p.tx.clone()))
                    .collect()
            }
            OrderingMode::Fifo => {
                // Return in order of addition (by timestamp in priority)
                let mut txs: Vec<_> = by_hash.values().collect();
                txs.sort_by_key(|p| p.added_at);
                txs.into_iter().take(max_count).map(|p| p.tx.clone()).collect()
            }
        }
    }

    /// Get transactions for a specific sender (ordered by nonce)
    pub async fn get_sender_transactions(&self, sender: &PublicKey) -> Vec<Transaction> {
        let by_hash = self.by_hash.read().await;
        let by_sender = self.by_sender.read().await;

        if let Some(hashes) = by_sender.get(sender) {
            let mut txs: Vec<_> = hashes
                .iter()
                .filter_map(|h| by_hash.get(h).map(|p| p.tx.clone()))
                .collect();
            txs.sort_by_key(|tx| tx.nonce);
            txs
        } else {
            Vec::new()
        }
    }

    /// Remove transactions that have been included in a block
    pub async fn remove_committed(&self, tx_hashes: &[Hash]) {
        let mut by_hash = self.by_hash.write().await;
        let mut by_sender = self.by_sender.write().await;
        let mut by_priority = self.by_priority.write().await;

        for hash in tx_hashes {
            self.remove_internal(hash, &mut by_hash, &mut by_sender, &mut by_priority);
        }
    }

    /// Remove expired transactions
    pub async fn remove_expired(&self) {
        let now = Self::current_timestamp();
        let expiry_threshold = now.saturating_sub(self.config.expiry_seconds);

        let mut by_hash = self.by_hash.write().await;
        let mut by_sender = self.by_sender.write().await;
        let mut by_priority = self.by_priority.write().await;

        let expired: Vec<Hash> = by_hash
            .iter()
            .filter(|(_, p)| p.added_at < expiry_threshold)
            .map(|(h, _)| *h)
            .collect();

        for hash in expired {
            self.remove_internal(&hash, &mut by_hash, &mut by_sender, &mut by_priority);
            warn!("Removed expired transaction {}", hash);
        }
    }

    /// Get current pool size
    pub async fn size(&self) -> usize {
        let by_hash = self.by_hash.read().await;
        by_hash.len()
    }

    /// Evict lowest priority transaction
    fn evict_lowest_priority(
        &self,
        by_hash: &mut HashMap<Hash, PendingTransaction>,
        by_sender: &mut HashMap<PublicKey, HashSet<Hash>>,
        by_priority: &mut BTreeMap<(TxPriority, Hash), Hash>,
    ) -> bool {
        if let Some(((priority, hash), _)) = by_priority.first_key_value() {
            let hash = *hash;
            let priority = *priority;
            by_priority.remove(&(priority, hash));
            if let Some(pending) = by_hash.remove(&hash) {
                if let Some(sender_txs) = by_sender.get_mut(&pending.tx.sender_pubkey) {
                    sender_txs.remove(&hash);
                }
            }
            warn!("Evicted lowest priority transaction {}", hash);
            true
        } else {
            false
        }
    }
}

/// Mempool errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum MempoolError {
    #[error("Transaction already exists in mempool")]
    AlreadyExists,

    #[error("Mempool is full")]
    PoolFull,

    #[error("Sender has reached transaction limit")]
    SenderLimitReached,

    #[error("Invalid transaction")]
    InvalidTransaction,
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{KeyPair, Op};

    fn create_test_tx(sender: &KeyPair, nonce: u64, fee: u64) -> Transaction {
        Transaction::new_signed(
            sender.public,
            nonce,
            fee,
            vec![Op::Transfer {
                to: KeyPair::generate().public,
                amount: 100,
            }],
            &sender.secret,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_add_and_get() {
        let mempool = Mempool::new(MempoolConfig::default());
        let sender = KeyPair::generate();
        let tx = create_test_tx(&sender, 1, 100);
        let hash = tx.hash().unwrap();

        mempool.add(tx.clone()).await.unwrap();

        let retrieved = mempool.get(&hash).await.unwrap();
        assert_eq!(retrieved.nonce, tx.nonce);
    }

    #[tokio::test]
    async fn test_duplicate_rejection() {
        let mempool = Mempool::new(MempoolConfig::default());
        let sender = KeyPair::generate();
        let tx = create_test_tx(&sender, 1, 100);

        mempool.add(tx.clone()).await.unwrap();
        let result = mempool.add(tx).await;

        assert!(matches!(result, Err(MempoolError::AlreadyExists)));
    }

    #[tokio::test]
    async fn test_remove() {
        let mempool = Mempool::new(MempoolConfig::default());
        let sender = KeyPair::generate();
        let tx = create_test_tx(&sender, 1, 100);
        let hash = tx.hash().unwrap();

        mempool.add(tx).await.unwrap();
        assert!(mempool.contains(&hash).await);

        mempool.remove(&hash).await;
        assert!(!mempool.contains(&hash).await);
    }

    #[tokio::test]
    async fn test_ordering_by_fee() {
        let config = MempoolConfig {
            ordering_mode: OrderingMode::FeeRate,
            ..Default::default()
        };
        let mempool = Mempool::new(config);
        let sender = KeyPair::generate();

        // Add transactions with different fees
        let tx_low = create_test_tx(&sender, 1, 10);
        let tx_high = create_test_tx(&sender, 2, 1000);
        let tx_med = create_test_tx(&sender, 3, 100);

        mempool.add(tx_low).await.unwrap();
        mempool.add(tx_high).await.unwrap();
        mempool.add(tx_med).await.unwrap();

        let txs = mempool.get_transactions(3).await;
        // Highest fee should come first
        assert_eq!(txs[0].fee, 1000);
    }

    #[tokio::test]
    async fn test_sender_limit() {
        let config = MempoolConfig {
            max_per_sender: 2,
            ..Default::default()
        };
        let mempool = Mempool::new(config);
        let sender = KeyPair::generate();

        mempool.add(create_test_tx(&sender, 1, 100)).await.unwrap();
        mempool.add(create_test_tx(&sender, 2, 100)).await.unwrap();
        let result = mempool.add(create_test_tx(&sender, 3, 100)).await;

        assert!(matches!(result, Err(MempoolError::SenderLimitReached)));
    }

    #[tokio::test]
    async fn test_get_sender_transactions() {
        let mempool = Mempool::new(MempoolConfig::default());
        let sender1 = KeyPair::generate();
        let sender2 = KeyPair::generate();

        mempool.add(create_test_tx(&sender1, 1, 100)).await.unwrap();
        mempool.add(create_test_tx(&sender1, 2, 100)).await.unwrap();
        mempool.add(create_test_tx(&sender2, 1, 100)).await.unwrap();

        let txs = mempool.get_sender_transactions(&sender1.public).await;
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].nonce, 1);
        assert_eq!(txs[1].nonce, 2);
    }
}
