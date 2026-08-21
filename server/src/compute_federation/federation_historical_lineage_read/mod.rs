//! Read-only adoption surface for sealed historical compute lineage carriers.

mod api;
mod mcp;
mod service;
mod transport;

#[cfg(test)]
mod source_contract_tests;

pub(crate) use api::routes;
pub(crate) use mcp::{admin_definitions, call_admin_if_handled, call_if_handled, definitions};
