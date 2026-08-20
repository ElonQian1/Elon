//! V278 immutable external-pool route-renewal receipt.
//!
//! This domain object records a renewed route closure. It grants no currentness by itself;
//! current execution authority is reconstructed only by the Store on an open transaction.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
