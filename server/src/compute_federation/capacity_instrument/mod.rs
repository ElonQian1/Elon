//! Immutable standardized capacity instruments and append-only lifecycle receipts.
//!
//! Registration and lifecycle changes do not publish prices, reserve capacity, move funds, or
//! settle work. An exact Offer adoption is the companion authority consumed by those later flows.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
