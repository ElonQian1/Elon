//! Full-root, pre-manifest inventory of semantic execution programs.
//!
//! This layer deliberately succeeds while programs are still missing. It commits every frozen
//! static member to one capability-normalized semantic program identity and records whether exact
//! source exists but still requires a real receipt. It never creates a quotient catalog,
//! `Supported` admission, dynamic member coverage, or Windows execution authority.

mod admission;
mod builder;
mod model;
mod review;

#[cfg(test)]
pub(super) use admission::provider_for_source_program_for_test;
pub(crate) use admission::ProgramCatalogAdmissionErrorV1;
pub(super) use admission::{
    review_lock_execution_program_inventory_v1, review_map_execution_program_inventory_v1,
    ProgramCatalogBindingV1, ProgramCatalogReceiptProviderV1, ReviewedExecutionProgramInventoryV1,
};
pub(super) use builder::{
    build_lock_execution_program_inventory_v1, build_map_execution_program_inventory_v1,
};
pub(super) use model::*;
