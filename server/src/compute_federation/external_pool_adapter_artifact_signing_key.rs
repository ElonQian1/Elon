//! Append-only trust-key contracts for future external-pool Adapter Artifact signatures.
//!
//! An active key is only an eligible signer root. It does not prove that any Artifact was signed,
//! inspected, adopted, loaded, or authorized for execution.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
