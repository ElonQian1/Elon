//! Inert lifecycle model for a future one-shot handle-bound SQLite session.
//!
//! This module has no session creation path and owns no routing table. It only defines the linear
//! phase, callback and handle-lease invariants that a later private VFS implementation must obey.
//! In particular, terminal quarantine has no transition back to an operational or retired phase.

#![allow(dead_code)]

mod state;
mod types;
