//! Immutable Start outbox authority, delivery evidence, and no-start proof facts.
//!
//! This domain contains no bearer token, secret, network client, Adapter resolver, or mutable
//! dispatch implementation. Store gates retain construction authority for every sealed wrapper.

mod canonical;
mod types;
mod validated;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validated::*;
