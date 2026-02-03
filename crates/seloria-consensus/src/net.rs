use seloria_core::{Block, PublicKey, Sig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeRequest {
    pub block: Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeResponse {
    pub validator_pubkey: PublicKey,
    pub signature: Sig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    pub block: Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
    pub status: String,
    pub height: u64,
    pub hash: String,
}
