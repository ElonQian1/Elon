//! Provider-neutral, server-run task-protocol conformance evidence.
//!
//! This Domain owns the canonical catalog and durable value objects only. It grants no Provider,
//! route, executor, activation, execution, usage, market, or settlement authority. Process
//! custody, execution carriers, and current-authority construction remain Store-private.

mod builders;
mod canonical;
mod catalog;
mod runtime_evidence;
mod types;
mod validation;

pub(crate) use builders::*;
pub(crate) use canonical::*;
pub(crate) use catalog::*;
pub(crate) use runtime_evidence::*;
pub(crate) use types::*;
pub(crate) use validation::*;
