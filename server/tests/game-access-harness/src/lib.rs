//! Production game authorization SQL and crypto; synthetic SQLite, no HTTP auth facade.
#![allow(dead_code)]
#[path = "../../../src/esk_platform/game_access/authority.rs"]
pub mod authority;
#[path = "../../../src/esk_platform/game_access/issue.rs"]
pub mod issue;
#[path = "../../../src/esk_platform/game_access/migration.rs"]
pub mod migration;
#[path = "../../../src/esk_platform/game_access/model.rs"]
pub mod model;
#[path = "../../../src/esk_platform/game_access/observe.rs"]
pub mod observe;
#[path = "../../../src/esk_platform/game_access/policy.rs"]
pub mod policy;
#[path = "../../../src/esk_platform/game_access/protocol/mod.rs"]
pub mod protocol;
#[path = "../../../src/esk_platform/game_access/revoke.rs"]
pub mod revoke;
#[cfg(test)]
#[path = "../../../src/esk_platform/game_access/test_support.rs"]
mod test_support;
#[cfg(test)]
mod tests;
