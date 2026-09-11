//! Direct production SQL/crypto tests with synthetic SQLite; no HTTP authentication facade.
#![allow(dead_code)]
#[path = "../../../src/esk_platform/game_rewards/authority.rs"]
pub mod authority;
#[path = "../../../src/esk_platform/game_rewards/funding.rs"]
pub mod funding;
#[cfg(test)]
mod interoperability;
#[path = "../../../src/esk_platform/game_rewards/ledger.rs"]
pub mod ledger;
#[path = "../../../src/esk_platform/game_rewards/migration.rs"]
pub mod migration;
#[path = "../../../src/esk_platform/game_rewards/model.rs"]
pub mod model;
#[path = "../../../src/esk_platform/game_rewards/policy.rs"]
pub mod policy;
#[cfg(test)]
#[path = "../../../src/esk_platform/game_rewards/tests.rs"]
mod tests;
