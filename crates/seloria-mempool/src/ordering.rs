use seloria_core::Transaction;

/// Transaction priority for ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TxPriority {
    /// Fee per byte (higher is better)
    pub fee_rate: u64,
    /// Submission timestamp (earlier is better for tie-breaking)
    pub timestamp: u64,
}

impl TxPriority {
    pub fn from_transaction(tx: &Transaction, timestamp: u64) -> Self {
        // Estimate transaction size (simplified)
        let estimated_size = estimate_tx_size(tx);
        let fee_rate = if estimated_size > 0 {
            tx.fee / estimated_size as u64
        } else {
            tx.fee
        };

        TxPriority {
            fee_rate,
            timestamp,
        }
    }
}

/// Estimate transaction size in bytes
fn estimate_tx_size(tx: &Transaction) -> usize {
    // Base: pubkey (32) + nonce (8) + fee (8) + signature (64) = 112
    let base_size = 112;

    // Estimate size of operations
    let ops_size: usize = tx.ops.iter().map(|op| estimate_op_size(op)).sum();

    base_size + ops_size
}

/// Estimate operation size
fn estimate_op_size(op: &seloria_core::Op) -> usize {
    use seloria_core::Op;

    match op {
        Op::AgentCertRegister { .. } => 256, // Certificate + signature
        Op::Transfer { .. } => 40,           // pubkey + amount
        Op::ClaimCreate { claim_type, .. } => 80 + claim_type.len(),
        Op::Attest { .. } => 48,             // claim_id + vote + stake
        Op::AppRegister { meta } => {
            128 + meta.version.len()
                + (meta.namespaces.len() * 32)
                + (meta.schemas.len() * 32)
                + (meta.recipes.len() * 32)
        }
        Op::KvPut { key, value, .. } => {
            48 + key.len() + estimate_kv_value_size(value)
        }
        Op::KvDel { key, .. } => 40 + key.len(),
        Op::KvAppend { key, value, .. } => {
            48 + key.len() + estimate_kv_value_size(value)
        }
        Op::NamespaceCreate { allowlist, .. } => 80 + allowlist.len() * 32,
    }
}

fn estimate_kv_value_size(value: &seloria_core::KvValue) -> usize {
    use seloria_core::KvData;

    let data_size = match &value.data {
        KvData::Inline(data) => data.len(),
        KvData::Reference { uri, .. } => 32 + uri.as_ref().map_or(0, |u| u.len()),
    };

    value.codec.len() + data_size
}

/// Ordering mode for transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderingMode {
    /// Order by fee rate (highest first)
    FeeRate,
    /// Order by timestamp (FIFO)
    Fifo,
}
