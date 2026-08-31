//! Rooted production-Windows `xShmLock` contract.
//!
//! The graph starts at the installed SQLite method slot.  Request ranges are represented by a
//! mechanically checked translation quotient: shared ranges have one one-slot representative and
//! exclusive ranges have one representative for each width.  No test fault controller or fixture
//! wrapper is part of this production graph.

mod builder;
mod dynamic;
mod input;
mod managed;
mod outcome;
mod range;

use super::{model::ContractGraph, poison};

pub(super) fn graph() -> ContractGraph {
    poison::validate_mutex_poison_absence();
    range::validate_translation_quotient();
    let mut graph = builder::Builder::new();
    let (root, requests) = input::build(&mut graph);
    for request in requests {
        managed::expand(&mut graph, request);
    }
    graph.finish(root)
}
