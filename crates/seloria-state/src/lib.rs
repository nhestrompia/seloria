//! Seloria State - State management and storage
//!
//! This crate provides chain state management, storage abstractions,
//! and merkle tree support.

pub mod error;
pub mod merkle;
pub mod state;
pub mod storage;

pub use error::StateError;
pub use merkle::compute_state_root;
pub use state::ChainState;
pub use storage::{FileStorage, MemoryStorage, Storage};
