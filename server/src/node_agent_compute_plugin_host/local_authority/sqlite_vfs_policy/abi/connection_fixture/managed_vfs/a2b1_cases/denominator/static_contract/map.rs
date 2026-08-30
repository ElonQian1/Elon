//! One rooted, production-Windows `xShmMap` denominator graph.
//!
//! The graph begins at the ABI null-write and replaces every production continuation through raw
//! admission, registry callback ownership, managed initialization/mapping, callback completion and
//! ABI projection.  Test-only fault wrappers and selectors never enter this module.

mod abi_raw;
mod builder;
mod expected;
mod loop_expansion;
mod managed;
mod projection;
mod registry;
mod witnesses;

use super::{model::ContractGraph, poison};
use builder::MapGraphBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MapMode {
    Observe,
    Extend,
}

impl MapMode {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Extend => "extend",
        }
    }
}

pub(super) fn graph() -> ContractGraph {
    poison::validate_mutex_poison_absence();
    loop_expansion::assert_authority_loop_bounds();
    let mut graph = MapGraphBuilder::new();
    let entries = abi_raw::build(&mut graph);
    registry::build(&mut graph, &entries.observe, MapMode::Observe);
    registry::build(&mut graph, &entries.extend, MapMode::Extend);
    graph.finish(abi_raw::ROOT)
}
