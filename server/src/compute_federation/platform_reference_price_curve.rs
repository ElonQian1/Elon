//! Canonical source-only contracts for a governed platform reference fallback curve batch.
//!
//! These DTOs grant no administrator role, market observation, v171 registration, capacity,
//! funds, Job, or Reservation authority. Independent review and Store-private application remain
//! separate boundaries.

mod canonical;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use types::*;
pub(crate) use validation::*;
