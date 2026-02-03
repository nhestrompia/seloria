use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid public key")]
    InvalidPublicKey,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Invalid hash length")]
    InvalidHashLength,

    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}
