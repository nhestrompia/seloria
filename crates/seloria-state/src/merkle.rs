use seloria_core::{hash_blake3, Hash};

/// Compute state root from key-value pairs
/// Uses sorted keys for determinism
pub fn compute_state_root<'a, I>(entries: I) -> Hash
where
    I: IntoIterator<Item = (&'a [u8], &'a [u8])>,
{
    let mut sorted: Vec<_> = entries.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    if sorted.is_empty() {
        return Hash::ZERO;
    }

    // Hash each key-value pair
    let leaf_hashes: Vec<Hash> = sorted
        .iter()
        .map(|(k, v)| {
            let mut data = Vec::with_capacity(k.len() + v.len());
            data.extend_from_slice(k);
            data.extend_from_slice(v);
            hash_blake3(&data)
        })
        .collect();

    // Build merkle tree
    merkle_root_from_hashes(&leaf_hashes)
}

/// Compute merkle root from a list of hashes
fn merkle_root_from_hashes(hashes: &[Hash]) -> Hash {
    if hashes.is_empty() {
        return Hash::ZERO;
    }

    if hashes.len() == 1 {
        return hashes[0];
    }

    let mut current_level = hashes.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for chunk in current_level.chunks(2) {
            let combined = if chunk.len() == 2 {
                let mut data = [0u8; 64];
                data[..32].copy_from_slice(chunk[0].as_bytes());
                data[32..].copy_from_slice(chunk[1].as_bytes());
                hash_blake3(&data)
            } else {
                // Odd number: duplicate last
                let mut data = [0u8; 64];
                data[..32].copy_from_slice(chunk[0].as_bytes());
                data[32..].copy_from_slice(chunk[0].as_bytes());
                hash_blake3(&data)
            };
            next_level.push(combined);
        }

        current_level = next_level;
    }

    current_level[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state_root() {
        let entries: Vec<(&[u8], &[u8])> = vec![];
        let root = compute_state_root(entries);
        assert_eq!(root, Hash::ZERO);
    }

    #[test]
    fn test_single_entry() {
        let entries = vec![(b"key".as_slice(), b"value".as_slice())];
        let root = compute_state_root(entries);
        assert_ne!(root, Hash::ZERO);
    }

    #[test]
    fn test_deterministic() {
        let entries1 = vec![
            (b"a".as_slice(), b"1".as_slice()),
            (b"b".as_slice(), b"2".as_slice()),
        ];
        let entries2 = vec![
            (b"b".as_slice(), b"2".as_slice()),
            (b"a".as_slice(), b"1".as_slice()),
        ];

        let root1 = compute_state_root(entries1);
        let root2 = compute_state_root(entries2);

        // Order shouldn't matter since we sort
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_different_values_different_root() {
        let entries1 = vec![(b"key".as_slice(), b"value1".as_slice())];
        let entries2 = vec![(b"key".as_slice(), b"value2".as_slice())];

        let root1 = compute_state_root(entries1);
        let root2 = compute_state_root(entries2);

        assert_ne!(root1, root2);
    }
}
