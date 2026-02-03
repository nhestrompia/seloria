//! Seloria VM - Transaction execution engine
//!
//! This crate provides transaction validation and execution logic.

pub mod error;
pub mod executor;
pub mod opcodes;
pub mod validation;

pub use error::VmError;
pub use executor::{ExecutionEvent, ExecutionResult, Executor};
pub use validation::{validate_transaction, ValidationResult};
