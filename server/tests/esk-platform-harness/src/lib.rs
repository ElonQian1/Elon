//! Executes production validation, migration and SQL against synthetic local SQLite.
//! This facade provides connections/IDs/time only; it does not authenticate HTTP
//! sessions, exercise the full migration chain, publish, or operate real balances.
#![allow(dead_code, unused_imports)]

pub(crate) mod esk_asset;

#[path = "../../../src/esk_asset_migration.rs"]
pub(crate) mod paper_migration;
#[path = "../../../src/esk_platform/migration.rs"]
pub(crate) mod platform_migration;
#[path = "../../../src/esk_platform/sellback/migration.rs"]
pub(crate) mod sellback_migration;
#[path = "../../../src/esk_platform/sui_address_binding/migration.rs"]
pub(crate) mod sui_address_binding_migration;

pub(crate) mod store;

#[cfg(test)]
mod tests;
