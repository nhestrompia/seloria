use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::crypto::Hash;
use crate::types::token::NATIVE_TOKEN_ID;

/// A unique identifier for a stake lock
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LockId(pub Hash);

impl LockId {
    pub fn new(hash: Hash) -> Self {
        LockId(hash)
    }
}

/// An account in the chain state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Account {
    /// Available balances per token
    pub balances: BTreeMap<Hash, u64>,
    /// Transaction nonce (incremented with each transaction)
    pub nonce: u64,
    /// Locked balances by lock ID (native token only, for claims)
    pub locked: BTreeMap<LockId, u64>,
}

impl Account {
    pub fn new_native(balance: u64) -> Self {
        let mut balances = BTreeMap::new();
        if balance > 0 {
            balances.insert(NATIVE_TOKEN_ID, balance);
        }
        Account {
            balances,
            nonce: 0,
            locked: BTreeMap::new(),
        }
    }

    /// Get available balance for a token
    pub fn balance(&self, token_id: &Hash) -> u64 {
        self.balances.get(token_id).copied().unwrap_or(0)
    }

    /// Get available native balance
    pub fn native_balance(&self) -> u64 {
        self.balance(&NATIVE_TOKEN_ID)
    }

    /// Get total native balance (available + locked)
    pub fn total_native_balance(&self) -> u64 {
        self.native_balance() + self.locked.values().sum::<u64>()
    }

    /// Credit a token balance
    pub fn credit(&mut self, token_id: &Hash, amount: u64) {
        if amount == 0 {
            return;
        }
        *self.balances.entry(*token_id).or_insert(0) += amount;
    }

    /// Debit a token balance (assumes caller has checked availability)
    pub fn debit(&mut self, token_id: &Hash, amount: u64) {
        if amount == 0 {
            return;
        }
        if let Some(balance) = self.balances.get_mut(token_id) {
            *balance -= amount;
            if *balance == 0 {
                self.balances.remove(token_id);
            }
        }
    }

    /// Lock a specific amount under a lock ID
    pub fn lock(&mut self, lock_id: LockId, amount: u64) -> bool {
        if self.native_balance() < amount {
            return false;
        }
        self.debit(&NATIVE_TOKEN_ID, amount);
        *self.locked.entry(lock_id).or_insert(0) += amount;
        true
    }

    /// Unlock and return amount to available balance
    pub fn unlock(&mut self, lock_id: &LockId) -> u64 {
        if let Some(amount) = self.locked.remove(lock_id) {
            self.credit(&NATIVE_TOKEN_ID, amount);
            amount
        } else {
            0
        }
    }

    /// Get locked amount for a specific lock ID
    pub fn get_locked(&self, lock_id: &LockId) -> u64 {
        self.locked.get(lock_id).copied().unwrap_or(0)
    }

    /// Slash locked amount (remove without returning to balance)
    pub fn slash_locked(&mut self, lock_id: &LockId, amount: u64) -> u64 {
        if let Some(locked) = self.locked.get_mut(lock_id) {
            let slashed = (*locked).min(amount);
            *locked -= slashed;
            if *locked == 0 {
                self.locked.remove(lock_id);
            }
            slashed
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hash_blake3;

    fn test_lock_id() -> LockId {
        LockId::new(hash_blake3(b"test"))
    }

    #[test]
    fn test_account_new() {
        let account = Account::new_native(1000);
        assert_eq!(account.native_balance(), 1000);
        assert_eq!(account.nonce, 0);
        assert!(account.locked.is_empty());
    }

    #[test]
    fn test_lock_and_unlock() {
        let mut account = Account::new_native(1000);
        let lock_id = test_lock_id();

        assert!(account.lock(lock_id, 300));
        assert_eq!(account.native_balance(), 700);
        assert_eq!(account.get_locked(&lock_id), 300);
        assert_eq!(account.total_native_balance(), 1000);

        let unlocked = account.unlock(&lock_id);
        assert_eq!(unlocked, 300);
        assert_eq!(account.native_balance(), 1000);
        assert_eq!(account.get_locked(&lock_id), 0);
    }

    #[test]
    fn test_lock_insufficient_balance() {
        let mut account = Account::new_native(100);
        let lock_id = test_lock_id();

        assert!(!account.lock(lock_id, 200));
        assert_eq!(account.native_balance(), 100);
    }

    #[test]
    fn test_slash_locked() {
        let mut account = Account::new_native(1000);
        let lock_id = test_lock_id();

        account.lock(lock_id, 500);
        let slashed = account.slash_locked(&lock_id, 100);
        assert_eq!(slashed, 100);
        assert_eq!(account.get_locked(&lock_id), 400);
        assert_eq!(account.total_native_balance(), 900);
    }
}
