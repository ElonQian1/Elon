//! Full-root, pre-manifest inventory of semantic execution programs.
//!
//! This layer deliberately succeeds while programs are still missing. It commits every frozen
//! static member to one capability-normalized semantic program identity and records whether exact
//! source exists but still requires a real receipt. It never creates a quotient catalog,
//! `Supported` admission, dynamic member coverage, or Windows execution authority.

mod builder;
mod model;

pub(super) use builder::build_map_execution_program_inventory_v1;
pub(super) use model::*;
