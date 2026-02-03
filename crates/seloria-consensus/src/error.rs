use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("Not the current leader")]
    NotLeader,

    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    #[error("Invalid quorum certificate: {0}")]
    InvalidQc(String),

    #[error("Insufficient signatures: have {have}, need {need}")]
    InsufficientSignatures { have: usize, need: usize },

    #[error("Invalid signature from validator")]
    InvalidSignature,

    #[error("Block height mismatch: expected {expected}, got {got}")]
    HeightMismatch { expected: u64, got: u64 },

    #[error("Previous hash mismatch")]
    PrevHashMismatch,

    #[error("Invalid state root")]
    InvalidStateRoot,

    #[error("Transaction execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Validator not found: {0}")]
    ValidatorNotFound(String),

    #[error("State error: {0}")]
    State(#[from] seloria_state::StateError),

    #[error("Core error: {0}")]
    Core(#[from] seloria_core::CoreError),

    #[error("VM error: {0}")]
    Vm(#[from] seloria_vm::VmError),
}
