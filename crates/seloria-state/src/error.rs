use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Claim not found: {0}")]
    ClaimNotFound(String),

    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),

    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Block already exists at height {0}")]
    BlockExists(u64),

    #[error("Invalid state root")]
    InvalidStateRoot,

    #[error("Core error: {0}")]
    Core(#[from] seloria_core::CoreError),
}
