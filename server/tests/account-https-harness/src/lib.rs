//! Exercise the production ingress policy without linking the full server test binary.
#![allow(dead_code)]

#[path = "../../../src/auth_request_guard.rs"]
mod auth_request_guard;
#[path = "../../../src/auth_safety_store.rs"]
mod auth_safety_store;
#[path = "../../../src/account_security/https/config.rs"]
mod config;
#[path = "../../../src/federated_auth_idempotency.rs"]
mod federated_auth_idempotency;
#[path = "../../../src/account_security/https/policy.rs"]
mod policy;
