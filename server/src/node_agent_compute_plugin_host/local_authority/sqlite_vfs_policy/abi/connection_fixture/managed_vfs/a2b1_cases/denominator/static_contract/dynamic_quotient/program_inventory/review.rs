//! Independently reviewed inventory identity.
//!
//! This file is intentionally excluded from both inventory-generation and projector source-scope
//! digests. Otherwise embedding the reviewed inventory digest here would make that inventory
//! recursively depend on itself. The quotient manifest binds this value as a separate field.

use super::super::super::source_leaf_authority::Digest32;

/// Filled only after the complete Map inventory has no planned-missing program and an independent
/// review has frozen its canonical bytes. `None` keeps catalog/manifest admission unreachable.
pub(super) const REVIEWED_MAP_EXECUTION_PROGRAM_INVENTORY_SHA256_V1: Option<Digest32> = None;

/// Filled only after the complete Lock inventory has no planned-missing program and an independent
/// review has frozen its canonical bytes. It is intentionally root-distinct from the Map review.
pub(super) const REVIEWED_LOCK_EXECUTION_PROGRAM_INVENTORY_SHA256_V1: Option<Digest32> = None;
