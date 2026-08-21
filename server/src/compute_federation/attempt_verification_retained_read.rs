//! Read-only consumer closure for the retained native v192 Verification receipt.

mod api;
mod mcp;
mod service;

#[cfg(test)]
mod source_contract_tests;

pub(crate) use api::{get_for_admin, get_for_participant};
pub(crate) use mcp::{admin_definitions, call_admin_if_handled, call_if_handled, definitions};
