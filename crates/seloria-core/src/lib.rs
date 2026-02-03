//! Seloria Core - Core types, cryptography, and serialization
//!
//! This crate provides the foundational types and utilities for the Seloria
//! agent-only blockchain.

pub mod crypto;
pub mod error;
pub mod serialize;
pub mod types;

pub use crypto::{hash_blake3, merkle_root, sign, verify, Hash, KeyPair, PublicKey, SecretKey, Sig};
pub use error::CoreError;
pub use types::*;
