//! Seloria Consensus - Block production and consensus
//!
//! This crate provides block building, proposing, validation, and
//! quorum certificate management.

pub mod block_builder;
pub mod events;
pub mod error;
pub mod net;
pub mod proposer;
pub mod qc;
pub mod validator;

pub use block_builder::{BlockBuilder, BlockBuilderConfig};
pub use events::BlockEventSink;
pub use error::ConsensusError;
pub use net::{CommitRequest, CommitResponse, ProposeRequest, ProposeResponse};
pub use proposer::{Proposer, ProposerConfig, ValidatorEndpoint};
pub use qc::{verify_qc, QcBuilder};
pub use validator::Validator;
