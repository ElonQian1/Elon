//! Offline accounting candidates. Source signatures attest facts; this crate cannot
//! establish exchange truth, authorize payments, or sign a main-project settlement.
mod calculate;
pub mod files;
pub mod model;
pub mod verify;

// Reuse the actual receiver's wire types instead of a parallel Settlement schema.
#[allow(dead_code)]
#[path = "../../../server/src/esk_platform/game_rewards/model.rs"]
pub mod settlement;

pub use calculate::prepare;

#[cfg(test)]
mod ledger_integration;
#[cfg(test)]
mod tests;
