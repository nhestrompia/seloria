use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("Insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },

    #[error("Agent not certified or certificate expired")]
    AgentNotCertified,

    #[error("Missing capability: {0:?}")]
    MissingCapability(seloria_core::Capability),

    #[error("Issuer not trusted: {0}")]
    IssuerNotTrusted(String),

    #[error("Claim not found: {0}")]
    ClaimNotFound(String),

    #[error("Claim already finalized")]
    ClaimAlreadyFinalized,

    #[error("Already attested to this claim")]
    AlreadyAttested,

    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),

    #[error("Namespace already exists: {0}")]
    NamespaceExists(String),

    #[error("Not authorized to write to namespace")]
    NamespaceUnauthorized,

    #[error("App already exists: {0}")]
    AppExists(String),

    #[error("Token not found: {0}")]
    TokenNotFound(String),

    #[error("Token already exists: {0}")]
    TokenExists(String),

    #[error("Pool not found: {0}")]
    PoolNotFound(String),

    #[error("Pool already exists: {0}")]
    PoolExists(String),

    #[error("Slippage exceeded")]
    SlippageExceeded,

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("State error: {0}")]
    State(#[from] seloria_state::StateError),

    #[error("Core error: {0}")]
    Core(#[from] seloria_core::CoreError),
}
