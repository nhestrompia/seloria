use std::collections::{BTreeMap, BTreeSet};

use seloria_core::{
    serialize, Account, AmmPool, AppMeta, Block, Claim, GenesisConfig, Hash, KvValue, LockId,
    NamespaceMeta, PublicKey, SignedAgentCertificate, TokenMeta, NATIVE_TOKEN_ID,
};
use tracing::{debug, info};

use crate::error::StateError;
use crate::merkle::compute_state_root;
use crate::storage::Storage;

/// Key prefixes for storage
mod keys {
    pub const ACCOUNT: &[u8] = b"acc:";
    pub const AGENT: &[u8] = b"agt:";
    pub const ISSUER: &[u8] = b"iss:";
    pub const CLAIM: &[u8] = b"clm:";
    pub const NAMESPACE: &[u8] = b"ns:";
    pub const KV: &[u8] = b"kv:";
    pub const APP: &[u8] = b"app:";
    pub const TOKEN: &[u8] = b"tok:";
    pub const POOL: &[u8] = b"pool:";
    pub const LP: &[u8] = b"lp:";
    pub const BLOCK: &[u8] = b"blk:";
    pub const TX: &[u8] = b"tx:";
    pub const CHAIN_ID: &[u8] = b"chain:id";
    pub const VALIDATORS: &[u8] = b"chain:validators";
    pub const HEAD: &[u8] = b"head";
}

/// The main chain state manager
pub struct ChainState<S: Storage> {
    storage: S,
    /// In-memory account cache
    pub accounts: BTreeMap<PublicKey, Account>,
    /// Registered agent certificates
    pub agent_registry: BTreeMap<PublicKey, SignedAgentCertificate>,
    /// Trusted certificate issuers
    pub trusted_issuers: BTreeSet<PublicKey>,
    /// Active claims
    pub claims: BTreeMap<Hash, Claim>,
    /// Namespace metadata
    pub namespaces: BTreeMap<Hash, NamespaceMeta>,
    /// KV store entries
    pub kv_store: BTreeMap<(Hash, String), KvValue>,
    /// Registered applications
    pub apps: BTreeMap<Hash, AppMeta>,
    /// Token registry
    pub tokens: BTreeMap<Hash, TokenMeta>,
    /// AMM pools
    pub pools: BTreeMap<Hash, AmmPool>,
    /// LP balances (pool_id, owner) -> amount
    pub lp_balances: BTreeMap<(Hash, PublicKey), u64>,
    /// Block storage by height
    pub blocks: BTreeMap<u64, Block>,
    /// Transaction index by hash
    pub tx_index: BTreeMap<Hash, seloria_core::Transaction>,
    /// Current head block
    pub head_block: Option<Block>,
    /// Current block height
    pub height: u64,
    /// Chain ID
    pub chain_id: u64,
    /// Validator public keys
    pub validators: Vec<PublicKey>,
}

impl<S: Storage + Clone> Clone for ChainState<S> {
    fn clone(&self) -> Self {
        ChainState {
            storage: self.storage.clone(),
            accounts: self.accounts.clone(),
            agent_registry: self.agent_registry.clone(),
            trusted_issuers: self.trusted_issuers.clone(),
            claims: self.claims.clone(),
            namespaces: self.namespaces.clone(),
            kv_store: self.kv_store.clone(),
            apps: self.apps.clone(),
            tokens: self.tokens.clone(),
            pools: self.pools.clone(),
            lp_balances: self.lp_balances.clone(),
            blocks: self.blocks.clone(),
            tx_index: self.tx_index.clone(),
            head_block: self.head_block.clone(),
            height: self.height,
            chain_id: self.chain_id,
            validators: self.validators.clone(),
        }
    }
}

impl<S: Storage> ChainState<S> {
    /// Create a new chain state with given storage
    pub fn new(storage: S) -> Self {
        ChainState {
            storage,
            accounts: BTreeMap::new(),
            agent_registry: BTreeMap::new(),
            trusted_issuers: BTreeSet::new(),
            claims: BTreeMap::new(),
            namespaces: BTreeMap::new(),
            kv_store: BTreeMap::new(),
            apps: BTreeMap::new(),
            tokens: BTreeMap::new(),
            pools: BTreeMap::new(),
            lp_balances: BTreeMap::new(),
            blocks: BTreeMap::new(),
            tx_index: BTreeMap::new(),
            head_block: None,
            height: 0,
            chain_id: 0,
            validators: Vec::new(),
        }
    }

    /// Initialize state from genesis configuration
    pub fn init_genesis(&mut self, config: &GenesisConfig) -> Result<(), StateError> {
        info!("Initializing genesis state");

        self.chain_id = config.chain_id;
        self.validators = config.validators.clone();

        // Set initial balances (native token)
        let mut native_supply: u64 = 0;
        for (pubkey, balance) in &config.initial_balances {
            let account = Account::new_native(*balance);
            self.accounts.insert(*pubkey, account);
            native_supply = native_supply.saturating_add(*balance);
            debug!("Set initial balance for {}: {}", pubkey, balance);
        }

        // Register native token
        let native = TokenMeta::native("Seloria", "SEL", 6, native_supply);
        self.tokens.insert(NATIVE_TOKEN_ID, native);

        // Set trusted issuers
        for issuer in &config.trusted_issuers {
            self.trusted_issuers.insert(*issuer);
            debug!("Added trusted issuer: {}", issuer);
        }

        // Create and store genesis block
        let genesis = config.create_genesis_block();
        self.blocks.insert(0, genesis.clone());
        self.head_block = Some(genesis);
        self.height = 0;

        // Persist to storage
        self.persist_state()?;

        info!("Genesis state initialized successfully");
        Ok(())
    }

    /// Persist current state to storage
    pub fn persist_state(&mut self) -> Result<(), StateError> {
        // Persist accounts
        for (pubkey, account) in &self.accounts {
            let key = [keys::ACCOUNT, pubkey.as_bytes()].concat();
            let value =
                serialize::to_bytes(account).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist agents
        for (pubkey, cert) in &self.agent_registry {
            let key = [keys::AGENT, pubkey.as_bytes()].concat();
            let value =
                serialize::to_bytes(cert).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist issuers
        for issuer in &self.trusted_issuers {
            let key = [keys::ISSUER, issuer.as_bytes()].concat();
            self.storage.put(&key, &[1u8]);
        }

        // Persist claims
        for (claim_id, claim) in &self.claims {
            let key = [keys::CLAIM, claim_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(claim).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist namespaces
        for (ns_id, ns) in &self.namespaces {
            let key = [keys::NAMESPACE, ns_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(ns).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist KV entries
        for ((ns_id, kv_key), value) in &self.kv_store {
            let storage_key = format_kv_key(ns_id, kv_key);
            let value =
                serialize::to_bytes(value).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&storage_key, &value);
        }

        // Persist apps
        for (app_id, app) in &self.apps {
            let key = [keys::APP, app_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(app).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist tokens
        for (token_id, token) in &self.tokens {
            let key = [keys::TOKEN, token_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(token).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist pools
        for (pool_id, pool) in &self.pools {
            let key = [keys::POOL, pool_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(pool).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist LP balances
        for ((pool_id, owner), amount) in &self.lp_balances {
            let key = format_lp_key(pool_id, owner);
            let value =
                serialize::to_bytes(amount).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist blocks
        for (height, block) in &self.blocks {
            let key = format_block_key(*height);
            let value =
                serialize::to_bytes(block).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist transaction index
        for (tx_hash, tx) in &self.tx_index {
            let key = format_tx_key(tx_hash);
            let value =
                serialize::to_bytes(tx).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(&key, &value);
        }

        // Persist head block
        if let Some(ref block) = self.head_block {
            let key = keys::HEAD;
            let value =
                serialize::to_bytes(block).map_err(|e| StateError::Serialization(e.to_string()))?;
            self.storage.put(key, &value);
        }

        // Persist chain metadata
        self.storage
            .put(keys::CHAIN_ID, &self.chain_id.to_le_bytes());
        let validators_bytes = serialize::to_bytes(&self.validators)
            .map_err(|e| StateError::Serialization(e.to_string()))?;
        self.storage.put(keys::VALIDATORS, &validators_bytes);

        self.storage.commit()?;
        Ok(())
    }

    /// Load state from storage into memory
    pub fn load_from_storage(&mut self) -> Result<(), StateError> {
        self.accounts.clear();
        self.agent_registry.clear();
        self.trusted_issuers.clear();
        self.claims.clear();
        self.namespaces.clear();
        self.kv_store.clear();
        self.apps.clear();
        self.tokens.clear();
        self.pools.clear();
        self.lp_balances.clear();
        self.blocks.clear();
        self.tx_index.clear();
        self.head_block = None;
        self.height = 0;

        for key in self.storage.keys_with_prefix(keys::ACCOUNT) {
            if let Some(value) = self.storage.get(&key) {
                let pk_bytes = &key[keys::ACCOUNT.len()..];
                if let Some(pubkey) = PublicKey::from_slice(pk_bytes) {
                    let account = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.accounts.insert(pubkey, account);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::AGENT) {
            if let Some(value) = self.storage.get(&key) {
                let pk_bytes = &key[keys::AGENT.len()..];
                if let Some(pubkey) = PublicKey::from_slice(pk_bytes) {
                    let cert = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.agent_registry.insert(pubkey, cert);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::ISSUER) {
            let pk_bytes = &key[keys::ISSUER.len()..];
            if let Some(pubkey) = PublicKey::from_slice(pk_bytes) {
                self.trusted_issuers.insert(pubkey);
            }
        }

        for key in self.storage.keys_with_prefix(keys::CLAIM) {
            if let Some(value) = self.storage.get(&key) {
                let claim_bytes = &key[keys::CLAIM.len()..];
                if let Some(claim_id) = Hash::from_slice(claim_bytes) {
                    let claim = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.claims.insert(claim_id, claim);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::NAMESPACE) {
            if let Some(value) = self.storage.get(&key) {
                let ns_bytes = &key[keys::NAMESPACE.len()..];
                if let Some(ns_id) = Hash::from_slice(ns_bytes) {
                    let ns = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.namespaces.insert(ns_id, ns);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::KV) {
            if let Some(value) = self.storage.get(&key) {
                let rest = &key[keys::KV.len()..];
                if rest.len() < 33 || rest[32] != b':' {
                    continue;
                }
                let ns_bytes = &rest[..32];
                let key_bytes = &rest[33..];
                if let Some(ns_id) = Hash::from_slice(ns_bytes) {
                    if let Ok(kv_key) = String::from_utf8(key_bytes.to_vec()) {
                        let kv_value = serialize::from_bytes(&value)
                            .map_err(|e| StateError::Serialization(e.to_string()))?;
                        self.kv_store.insert((ns_id, kv_key), kv_value);
                    }
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::APP) {
            if let Some(value) = self.storage.get(&key) {
                let app_bytes = &key[keys::APP.len()..];
                if let Some(app_id) = Hash::from_slice(app_bytes) {
                    let app = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.apps.insert(app_id, app);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::TOKEN) {
            if let Some(value) = self.storage.get(&key) {
                let token_bytes = &key[keys::TOKEN.len()..];
                if let Some(token_id) = Hash::from_slice(token_bytes) {
                    let token = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.tokens.insert(token_id, token);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::POOL) {
            if let Some(value) = self.storage.get(&key) {
                let pool_bytes = &key[keys::POOL.len()..];
                if let Some(pool_id) = Hash::from_slice(pool_bytes) {
                    let pool = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.pools.insert(pool_id, pool);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::LP) {
            if let Some(value) = self.storage.get(&key) {
                let rest = &key[keys::LP.len()..];
                if rest.len() != 64 {
                    continue;
                }
                let pool_bytes = &rest[..32];
                let owner_bytes = &rest[32..];
                if let (Some(pool_id), Some(owner)) =
                    (Hash::from_slice(pool_bytes), PublicKey::from_slice(owner_bytes))
                {
                    let amount: u64 = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.lp_balances.insert((pool_id, owner), amount);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::BLOCK) {
            if let Some(value) = self.storage.get(&key) {
                let height_bytes = &key[keys::BLOCK.len()..];
                if height_bytes.len() == 8 {
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(height_bytes);
                    let height = u64::from_le_bytes(arr);
                    let block = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.blocks.insert(height, block);
                }
            }
        }

        for key in self.storage.keys_with_prefix(keys::TX) {
            if let Some(value) = self.storage.get(&key) {
                let tx_bytes = &key[keys::TX.len()..];
                if let Some(tx_hash) = Hash::from_slice(tx_bytes) {
                    let tx = serialize::from_bytes(&value)
                        .map_err(|e| StateError::Serialization(e.to_string()))?;
                    self.tx_index.insert(tx_hash, tx);
                }
            }
        }

        if let Some(value) = self.storage.get(keys::HEAD) {
            let block: Block = serialize::from_bytes(&value)
                .map_err(|e| StateError::Serialization(e.to_string()))?;
            self.height = block.header.height;
            self.head_block = Some(block);
        }

        if let Some(value) = self.storage.get(keys::CHAIN_ID) {
            if value.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&value);
                self.chain_id = u64::from_le_bytes(arr);
            }
        } else if let Some(ref head) = self.head_block {
            self.chain_id = head.header.chain_id;
        }

        if let Some(value) = self.storage.get(keys::VALIDATORS) {
            let validators = serialize::from_bytes(&value)
                .map_err(|e| StateError::Serialization(e.to_string()))?;
            self.validators = validators;
        }

        Ok(())
    }

    /// Compute the current state root
    pub fn compute_state_root(&self) -> Result<Hash, StateError> {
        let mut entries: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();

        // Add accounts
        for (pubkey, account) in &self.accounts {
            let key = [keys::ACCOUNT, pubkey.as_bytes()].concat();
            let value =
                serialize::to_bytes(account).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        // Add agents
        for (pubkey, cert) in &self.agent_registry {
            let key = [keys::AGENT, pubkey.as_bytes()].concat();
            let value =
                serialize::to_bytes(cert).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        // Add claims
        for (claim_id, claim) in &self.claims {
            let key = [keys::CLAIM, claim_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(claim).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        // Add namespaces
        for (ns_id, ns) in &self.namespaces {
            let key = [keys::NAMESPACE, ns_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(ns).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        // Add KV entries
        for ((ns_id, kv_key), value) in &self.kv_store {
            let storage_key = format_kv_key(ns_id, kv_key);
            let value =
                serialize::to_bytes(value).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((storage_key, value));
        }

        // Add apps
        for (app_id, app) in &self.apps {
            let key = [keys::APP, app_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(app).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        // Add tokens
        for (token_id, token) in &self.tokens {
            let key = [keys::TOKEN, token_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(token).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        // Add pools
        for (pool_id, pool) in &self.pools {
            let key = [keys::POOL, pool_id.as_bytes()].concat();
            let value =
                serialize::to_bytes(pool).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        // Add LP balances
        for ((pool_id, owner), amount) in &self.lp_balances {
            let key = format_lp_key(pool_id, owner);
            let value =
                serialize::to_bytes(amount).map_err(|e| StateError::Serialization(e.to_string()))?;
            entries.push((key, value));
        }

        let entry_refs: Vec<(&[u8], &[u8])> = entries
            .iter()
            .map(|(k, v)| (k.as_slice(), v.as_slice()))
            .collect();

        Ok(compute_state_root(entry_refs))
    }

    // Account operations

    /// Get account, creating default if not exists
    pub fn get_or_create_account(&mut self, pubkey: &PublicKey) -> &mut Account {
        self.accounts
            .entry(*pubkey)
            .or_insert_with(Account::default)
    }

    /// Get account (read-only)
    pub fn get_account(&self, pubkey: &PublicKey) -> Option<&Account> {
        self.accounts.get(pubkey)
    }

    /// Get account balance
    pub fn get_balance(&self, pubkey: &PublicKey) -> u64 {
        self.get_token_balance(pubkey, &NATIVE_TOKEN_ID)
    }

    /// Get token balance
    pub fn get_token_balance(&self, pubkey: &PublicKey, token_id: &Hash) -> u64 {
        self.accounts
            .get(pubkey)
            .map_or(0, |a| a.balance(token_id))
    }

    /// Credit a token balance
    pub fn credit_token(&mut self, pubkey: &PublicKey, token_id: &Hash, amount: u64) {
        let account = self.get_or_create_account(pubkey);
        account.credit(token_id, amount);
    }

    /// Debit a token balance
    pub fn debit_token(
        &mut self,
        pubkey: &PublicKey,
        token_id: &Hash,
        amount: u64,
    ) -> Result<(), StateError> {
        let balance = self.get_token_balance(pubkey, token_id);
        if balance < amount {
            return Err(StateError::InsufficientBalance {
                have: balance,
                need: amount,
            });
        }
        let account = self.get_or_create_account(pubkey);
        account.debit(token_id, amount);
        Ok(())
    }

    /// Transfer native tokens between accounts
    pub fn transfer(
        &mut self,
        from: &PublicKey,
        to: &PublicKey,
        amount: u64,
    ) -> Result<(), StateError> {
        self.transfer_token(from, to, &NATIVE_TOKEN_ID, amount)
    }

    /// Transfer tokens between accounts
    pub fn transfer_token(
        &mut self,
        from: &PublicKey,
        to: &PublicKey,
        token_id: &Hash,
        amount: u64,
    ) -> Result<(), StateError> {
        // Check balance
        let from_balance = self.get_token_balance(from, token_id);
        if from_balance < amount {
            return Err(StateError::InsufficientBalance {
                have: from_balance,
                need: amount,
            });
        }

        self.get_or_create_account(from).debit(token_id, amount);
        self.get_or_create_account(to).credit(token_id, amount);

        Ok(())
    }

    /// Deduct fee from account
    pub fn deduct_fee(&mut self, pubkey: &PublicKey, fee: u64) -> Result<(), StateError> {
        let account = self.get_or_create_account(pubkey);
        if account.native_balance() < fee {
            return Err(StateError::InsufficientBalance {
                have: account.native_balance(),
                need: fee,
            });
        }
        account.debit(&NATIVE_TOKEN_ID, fee);
        Ok(())
    }

    /// Distribute fee to validators (equal split, remainder to first validator)
    pub fn distribute_fee_to_validators(&mut self, fee: u64) {
        if fee == 0 || self.validators.is_empty() {
            return;
        }

        let validators = self.validators.clone();
        let count = validators.len() as u64;
        let share = fee / count;
        let remainder = fee % count;

        for (idx, validator) in validators.iter().enumerate() {
            let mut credit = share;
            if remainder > 0 && idx == 0 {
                credit += remainder;
            }
            if credit > 0 {
                self.get_or_create_account(validator)
                    .credit(&NATIVE_TOKEN_ID, credit);
            }
        }
    }

    /// Increment account nonce
    pub fn increment_nonce(&mut self, pubkey: &PublicKey) {
        self.get_or_create_account(pubkey).nonce += 1;
    }

    /// Lock tokens for a claim
    pub fn lock_stake(
        &mut self,
        pubkey: &PublicKey,
        lock_id: LockId,
        amount: u64,
    ) -> Result<(), StateError> {
        let account = self.get_or_create_account(pubkey);
        if !account.lock(lock_id, amount) {
            return Err(StateError::InsufficientBalance {
                have: account.native_balance(),
                need: amount,
            });
        }
        Ok(())
    }

    /// Unlock and return stake to account
    pub fn unlock_stake(&mut self, pubkey: &PublicKey, lock_id: &LockId) -> u64 {
        if let Some(account) = self.accounts.get_mut(pubkey) {
            account.unlock(lock_id)
        } else {
            0
        }
    }

    // Agent operations

    /// Register an agent certificate
    pub fn register_agent(&mut self, cert: SignedAgentCertificate) {
        self.agent_registry.insert(cert.cert.agent_pubkey, cert);
    }

    /// Get agent certificate
    pub fn get_agent(&self, pubkey: &PublicKey) -> Option<&SignedAgentCertificate> {
        self.agent_registry.get(pubkey)
    }

    /// Check if pubkey is a certified agent (not expired)
    pub fn is_certified_agent(&self, pubkey: &PublicKey, current_time: u64) -> bool {
        self.agent_registry
            .get(pubkey)
            .map_or(false, |cert| !cert.is_expired(current_time))
    }

    /// Check if pubkey is a trusted issuer
    pub fn is_trusted_issuer(&self, pubkey: &PublicKey) -> bool {
        self.trusted_issuers.contains(pubkey)
    }

    // Claim operations

    /// Add a new claim
    pub fn add_claim(&mut self, claim: Claim) {
        self.claims.insert(claim.id, claim);
    }

    /// Get a claim by ID
    pub fn get_claim(&self, claim_id: &Hash) -> Option<&Claim> {
        self.claims.get(claim_id)
    }

    /// Get a mutable claim
    pub fn get_claim_mut(&mut self, claim_id: &Hash) -> Option<&mut Claim> {
        self.claims.get_mut(claim_id)
    }

    // Namespace operations

    /// Add a new namespace
    pub fn add_namespace(&mut self, ns: NamespaceMeta) {
        self.namespaces.insert(ns.ns_id, ns);
    }

    /// Get namespace metadata
    pub fn get_namespace(&self, ns_id: &Hash) -> Option<&NamespaceMeta> {
        self.namespaces.get(ns_id)
    }

    // KV operations

    /// Put a KV entry
    pub fn kv_put(&mut self, ns_id: Hash, key: String, value: KvValue) {
        self.kv_store.insert((ns_id, key), value);
    }

    /// Get a KV entry
    pub fn kv_get(&self, ns_id: &Hash, key: &str) -> Option<&KvValue> {
        self.kv_store.get(&(*ns_id, key.to_string()))
    }

    /// Delete a KV entry
    pub fn kv_delete(&mut self, ns_id: &Hash, key: &str) -> Option<KvValue> {
        self.kv_store.remove(&(*ns_id, key.to_string()))
    }

    /// Get all keys in a namespace
    pub fn kv_keys(&self, ns_id: &Hash) -> Vec<String> {
        self.kv_store
            .keys()
            .filter_map(|(nid, k)| {
                if nid == ns_id {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    // App operations

    /// Register an app
    pub fn register_app(&mut self, app: AppMeta) {
        self.apps.insert(app.app_id, app);
    }

    /// Get an app by ID
    pub fn get_app(&self, app_id: &Hash) -> Option<&AppMeta> {
        self.apps.get(app_id)
    }

    // Token operations

    /// Register a new token
    pub fn add_token(&mut self, token: TokenMeta) {
        self.tokens.insert(token.token_id, token);
    }

    /// Get token metadata
    pub fn get_token(&self, token_id: &Hash) -> Option<&TokenMeta> {
        self.tokens.get(token_id)
    }

    // AMM pool operations

    /// Add a new pool
    pub fn add_pool(&mut self, pool: AmmPool) {
        self.pools.insert(pool.pool_id, pool);
    }

    /// Get pool metadata
    pub fn get_pool(&self, pool_id: &Hash) -> Option<&AmmPool> {
        self.pools.get(pool_id)
    }

    /// Get mutable pool
    pub fn get_pool_mut(&mut self, pool_id: &Hash) -> Option<&mut AmmPool> {
        self.pools.get_mut(pool_id)
    }

    /// Get LP balance
    pub fn get_lp_balance(&self, pool_id: &Hash, owner: &PublicKey) -> u64 {
        self.lp_balances
            .get(&(*pool_id, *owner))
            .copied()
            .unwrap_or(0)
    }

    /// Credit LP balance
    pub fn credit_lp(&mut self, pool_id: &Hash, owner: &PublicKey, amount: u64) {
        if amount == 0 {
            return;
        }
        *self
            .lp_balances
            .entry((*pool_id, *owner))
            .or_insert(0) += amount;
    }

    /// Debit LP balance
    pub fn debit_lp(
        &mut self,
        pool_id: &Hash,
        owner: &PublicKey,
        amount: u64,
    ) -> Result<(), StateError> {
        let balance = self.get_lp_balance(pool_id, owner);
        if balance < amount {
            return Err(StateError::InsufficientBalance {
                have: balance,
                need: amount,
            });
        }
        if let Some(balance_mut) = self.lp_balances.get_mut(&(*pool_id, *owner)) {
            *balance_mut -= amount;
            if *balance_mut == 0 {
                self.lp_balances.remove(&(*pool_id, *owner));
            }
        }
        Ok(())
    }

    // Block and transaction index operations

    /// Get a block by height
    pub fn get_block(&self, height: u64) -> Option<&Block> {
        self.blocks.get(&height)
    }

    /// Get a transaction by hash
    pub fn get_transaction(&self, tx_hash: &Hash) -> Option<&seloria_core::Transaction> {
        self.tx_index.get(tx_hash)
    }

    // Block operations

    /// Apply a block to the state
    pub fn apply_block(&mut self, block: Block) -> Result<(), StateError> {
        if block.header.height != self.height + 1 && self.height > 0 {
            return Err(StateError::BlockExists(block.header.height));
        }

        // Index transactions
        for tx in &block.txs {
            if let Ok(hash) = tx.hash() {
                self.tx_index.insert(hash, tx.clone());
            }
        }

        self.blocks.insert(block.header.height, block.clone());
        self.head_block = Some(block);
        self.height += 1;

        Ok(())
    }

    /// Get current block height
    pub fn current_height(&self) -> u64 {
        self.height
    }

    /// Rollback storage changes
    pub fn rollback(&mut self) {
        self.storage.rollback();
    }
}

/// Format KV storage key
fn format_kv_key(ns_id: &Hash, key: &str) -> Vec<u8> {
    let mut storage_key = keys::KV.to_vec();
    storage_key.extend_from_slice(ns_id.as_bytes());
    storage_key.push(b':');
    storage_key.extend_from_slice(key.as_bytes());
    storage_key
}

/// Format LP balance storage key
fn format_lp_key(pool_id: &Hash, owner: &PublicKey) -> Vec<u8> {
    let mut key = keys::LP.to_vec();
    key.extend_from_slice(pool_id.as_bytes());
    key.extend_from_slice(owner.as_bytes());
    key
}

/// Format block storage key
fn format_block_key(height: u64) -> Vec<u8> {
    let mut key = keys::BLOCK.to_vec();
    key.extend_from_slice(&height.to_le_bytes());
    key
}

/// Format transaction storage key
fn format_tx_key(tx_hash: &Hash) -> Vec<u8> {
    let mut key = keys::TX.to_vec();
    key.extend_from_slice(tx_hash.as_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use seloria_core::{hash_blake3, KeyPair};

    fn create_test_state() -> ChainState<MemoryStorage> {
        ChainState::new(MemoryStorage::new())
    }

    #[test]
    fn test_genesis_initialization() {
        let mut state = create_test_state();
        let issuer = KeyPair::generate();
        let validator = KeyPair::generate();
        let user = KeyPair::generate();

        let config = GenesisConfig {
            chain_id: 1,
            timestamp: 0,
            initial_balances: vec![(user.public, 1_000_000)],
            trusted_issuers: vec![issuer.public],
            validators: vec![validator.public],
        };

        state.init_genesis(&config).unwrap();

        assert_eq!(state.get_balance(&user.public), 1_000_000);
        assert!(state.is_trusted_issuer(&issuer.public));
        assert_eq!(state.height, 0);
    }

    #[test]
    fn test_transfer() {
        let mut state = create_test_state();
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        state.credit_token(&alice.public, &NATIVE_TOKEN_ID, 1000);

        state.transfer(&alice.public, &bob.public, 300).unwrap();

        assert_eq!(state.get_balance(&alice.public), 700);
        assert_eq!(state.get_balance(&bob.public), 300);
    }

    #[test]
    fn test_transfer_insufficient_balance() {
        let mut state = create_test_state();
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        state.credit_token(&alice.public, &NATIVE_TOKEN_ID, 100);

        let result = state.transfer(&alice.public, &bob.public, 200);
        assert!(matches!(result, Err(StateError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_distribute_fee_to_validators() {
        let mut state = create_test_state();
        let v1 = KeyPair::generate();
        let v2 = KeyPair::generate();
        state.validators = vec![v1.public, v2.public];

        state.distribute_fee_to_validators(101);

        assert_eq!(state.get_balance(&v1.public), 51);
        assert_eq!(state.get_balance(&v2.public), 50);
    }

    #[test]
    fn test_stake_locking() {
        let mut state = create_test_state();
        let user = KeyPair::generate();
        let lock_id = LockId::new(hash_blake3(b"claim1"));

        state.credit_token(&user.public, &NATIVE_TOKEN_ID, 1000);

        state.lock_stake(&user.public, lock_id, 300).unwrap();
        assert_eq!(state.get_balance(&user.public), 700);

        let unlocked = state.unlock_stake(&user.public, &lock_id);
        assert_eq!(unlocked, 300);
        assert_eq!(state.get_balance(&user.public), 1000);
    }

    #[test]
    fn test_state_root_deterministic() {
        let mut state = create_test_state();
        let user = KeyPair::generate();

        state.credit_token(&user.public, &NATIVE_TOKEN_ID, 1000);

        let root1 = state.compute_state_root().unwrap();
        let root2 = state.compute_state_root().unwrap();

        assert_eq!(root1, root2);
    }

    #[test]
    fn test_state_root_changes() {
        let mut state = create_test_state();
        let user = KeyPair::generate();

        state.credit_token(&user.public, &NATIVE_TOKEN_ID, 1000);
        let root1 = state.compute_state_root().unwrap();

        state.credit_token(&user.public, &NATIVE_TOKEN_ID, 1000);
        let root2 = state.compute_state_root().unwrap();

        assert_ne!(root1, root2);
    }
}
