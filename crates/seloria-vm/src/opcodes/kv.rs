use seloria_core::{Hash, KvData, KvValue, NamespaceMeta, NamespacePolicy, PublicKey};
use seloria_state::{ChainState, Storage};
use tracing::debug;

use crate::error::VmError;

/// Execute NAMESPACE_CREATE operation
pub fn execute_namespace_create<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    ns_id: &Hash,
    policy: NamespacePolicy,
    allowlist: Vec<PublicKey>,
    min_write_stake: u64,
) -> Result<(), VmError> {
    // Check namespace doesn't already exist
    if state.get_namespace(ns_id).is_some() {
        return Err(VmError::NamespaceExists(ns_id.to_hex()));
    }

    // Create namespace metadata
    let ns_meta = NamespaceMeta {
        ns_id: *ns_id,
        owner: *sender,
        policy,
        allowlist,
        min_write_stake,
    };

    state.add_namespace(ns_meta);

    debug!("Created namespace {} owned by {}", ns_id, sender);

    Ok(())
}

/// Execute KV_PUT operation
pub fn execute_kv_put<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    ns_id: &Hash,
    key: &str,
    value: KvValue,
) -> Result<(), VmError> {
    // Check namespace exists
    let ns = state
        .get_namespace(ns_id)
        .ok_or_else(|| VmError::NamespaceNotFound(ns_id.to_hex()))?;

    // Check write permission
    let sender_balance = state.get_balance(sender);
    if !ns.can_write(sender, sender_balance) {
        return Err(VmError::NamespaceUnauthorized);
    }

    // Put the value
    state.kv_put(*ns_id, key.to_string(), value);

    debug!("Put key '{}' in namespace {}", key, ns_id);

    Ok(())
}

/// Execute KV_DEL operation
pub fn execute_kv_del<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    ns_id: &Hash,
    key: &str,
) -> Result<(), VmError> {
    // Check namespace exists
    let ns = state
        .get_namespace(ns_id)
        .ok_or_else(|| VmError::NamespaceNotFound(ns_id.to_hex()))?;

    // Check write permission
    let sender_balance = state.get_balance(sender);
    if !ns.can_write(sender, sender_balance) {
        return Err(VmError::NamespaceUnauthorized);
    }

    // Check key exists
    if state.kv_get(ns_id, key).is_none() {
        return Err(VmError::KeyNotFound(key.to_string()));
    }

    // Delete the key
    state.kv_delete(ns_id, key);

    debug!("Deleted key '{}' from namespace {}", key, ns_id);

    Ok(())
}

/// Execute KV_APPEND operation
pub fn execute_kv_append<S: Storage>(
    state: &mut ChainState<S>,
    sender: &PublicKey,
    ns_id: &Hash,
    key: &str,
    value: KvValue,
) -> Result<(), VmError> {
    // Check namespace exists
    let ns = state
        .get_namespace(ns_id)
        .ok_or_else(|| VmError::NamespaceNotFound(ns_id.to_hex()))?;

    // Check write permission
    let sender_balance = state.get_balance(sender);
    if !ns.can_write(sender, sender_balance) {
        return Err(VmError::NamespaceUnauthorized);
    }

    // Get existing value or create empty
    let existing = state.kv_get(ns_id, key).cloned();

    let new_value = match existing {
        Some(existing_val) => {
            // Append to existing inline data
            match (existing_val.data, &value.data) {
                (KvData::Inline(mut existing_data), KvData::Inline(new_data)) => {
                    existing_data.extend_from_slice(new_data);
                    KvValue {
                        codec: existing_val.codec,
                        data: KvData::Inline(existing_data),
                    }
                }
                _ => {
                    // Can't append to/with references, just replace
                    value
                }
            }
        }
        None => value,
    };

    state.kv_put(*ns_id, key.to_string(), new_value);

    debug!("Appended to key '{}' in namespace {}", key, ns_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_core::{hash_blake3, KeyPair};
    use seloria_state::MemoryStorage;

    fn setup_state_with_namespace() -> (ChainState<MemoryStorage>, KeyPair, Hash) {
        let mut state = ChainState::new(MemoryStorage::new());
        let owner = KeyPair::generate();
        let ns_id = hash_blake3(b"test namespace");

        state.get_or_create_account(&owner.public).balance = 10000;

        execute_namespace_create(
            &mut state,
            &owner.public,
            &ns_id,
            NamespacePolicy::OwnerOnly,
            vec![],
            0,
        )
        .unwrap();

        (state, owner, ns_id)
    }

    #[test]
    fn test_namespace_create() {
        let mut state = ChainState::new(MemoryStorage::new());
        let owner = KeyPair::generate();
        let ns_id = hash_blake3(b"test namespace");

        execute_namespace_create(
            &mut state,
            &owner.public,
            &ns_id,
            NamespacePolicy::OwnerOnly,
            vec![],
            0,
        )
        .unwrap();

        let ns = state.get_namespace(&ns_id).unwrap();
        assert_eq!(ns.owner, owner.public);
        assert!(matches!(ns.policy, NamespacePolicy::OwnerOnly));
    }

    #[test]
    fn test_namespace_create_duplicate() {
        let (mut state, owner, ns_id) = setup_state_with_namespace();

        let result = execute_namespace_create(
            &mut state,
            &owner.public,
            &ns_id,
            NamespacePolicy::OwnerOnly,
            vec![],
            0,
        );

        assert!(matches!(result, Err(VmError::NamespaceExists(_))));
    }

    #[test]
    fn test_kv_put() {
        let (mut state, owner, ns_id) = setup_state_with_namespace();

        let value = KvValue::inline("json", b"{\"key\": \"value\"}".to_vec());
        execute_kv_put(&mut state, &owner.public, &ns_id, "test_key", value).unwrap();

        let retrieved = state.kv_get(&ns_id, "test_key").unwrap();
        assert_eq!(retrieved.codec, "json");
    }

    #[test]
    fn test_kv_put_unauthorized() {
        let (mut state, _, ns_id) = setup_state_with_namespace();
        let other = KeyPair::generate();

        let value = KvValue::inline("json", b"{}".to_vec());
        let result = execute_kv_put(&mut state, &other.public, &ns_id, "test_key", value);

        assert!(matches!(result, Err(VmError::NamespaceUnauthorized)));
    }

    #[test]
    fn test_kv_del() {
        let (mut state, owner, ns_id) = setup_state_with_namespace();

        let value = KvValue::inline("json", b"{}".to_vec());
        execute_kv_put(&mut state, &owner.public, &ns_id, "test_key", value).unwrap();

        execute_kv_del(&mut state, &owner.public, &ns_id, "test_key").unwrap();

        assert!(state.kv_get(&ns_id, "test_key").is_none());
    }

    #[test]
    fn test_kv_del_not_found() {
        let (mut state, owner, ns_id) = setup_state_with_namespace();

        let result = execute_kv_del(&mut state, &owner.public, &ns_id, "nonexistent");
        assert!(matches!(result, Err(VmError::KeyNotFound(_))));
    }

    #[test]
    fn test_kv_append() {
        let (mut state, owner, ns_id) = setup_state_with_namespace();

        let value1 = KvValue::inline("raw", b"hello".to_vec());
        execute_kv_put(&mut state, &owner.public, &ns_id, "test_key", value1).unwrap();

        let value2 = KvValue::inline("raw", b" world".to_vec());
        execute_kv_append(&mut state, &owner.public, &ns_id, "test_key", value2).unwrap();

        let retrieved = state.kv_get(&ns_id, "test_key").unwrap();
        match &retrieved.data {
            KvData::Inline(data) => assert_eq!(data, b"hello world"),
            _ => panic!("Expected inline data"),
        }
    }

    #[test]
    fn test_allowlist_policy() {
        let mut state = ChainState::new(MemoryStorage::new());
        let owner = KeyPair::generate();
        let allowed = KeyPair::generate();
        let denied = KeyPair::generate();
        let ns_id = hash_blake3(b"allowlist namespace");

        execute_namespace_create(
            &mut state,
            &owner.public,
            &ns_id,
            NamespacePolicy::Allowlist,
            vec![allowed.public],
            0,
        )
        .unwrap();

        // Allowed user can write
        let value = KvValue::inline("raw", b"test".to_vec());
        assert!(execute_kv_put(&mut state, &allowed.public, &ns_id, "key", value.clone()).is_ok());

        // Denied user cannot write
        assert!(
            matches!(
                execute_kv_put(&mut state, &denied.public, &ns_id, "key2", value),
                Err(VmError::NamespaceUnauthorized)
            )
        );
    }

    #[test]
    fn test_stake_gated_policy() {
        let mut state = ChainState::new(MemoryStorage::new());
        let owner = KeyPair::generate();
        let rich = KeyPair::generate();
        let poor = KeyPair::generate();
        let ns_id = hash_blake3(b"stake gated namespace");

        state.get_or_create_account(&rich.public).balance = 1000;
        state.get_or_create_account(&poor.public).balance = 100;

        execute_namespace_create(
            &mut state,
            &owner.public,
            &ns_id,
            NamespacePolicy::StakeGated,
            vec![],
            500, // Min 500 tokens to write
        )
        .unwrap();

        // Rich user can write
        let value = KvValue::inline("raw", b"test".to_vec());
        assert!(execute_kv_put(&mut state, &rich.public, &ns_id, "key", value.clone()).is_ok());

        // Poor user cannot write
        assert!(
            matches!(
                execute_kv_put(&mut state, &poor.public, &ns_id, "key2", value),
                Err(VmError::NamespaceUnauthorized)
            )
        );
    }
}
