//! Exercise the production ingress policy without linking the full server test binary.
#![allow(dead_code)]

#[path = "../../../src/account_security/https/acme/mod.rs"]
mod acme;

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
#[path = "../../../src/router/quant_http_preview.rs"]
pub mod quant_http_preview;
#[path = "../../../src/account_security/https/quant_public.rs"]
mod quant_public;
mod router {
    pub(crate) use crate::quant_http_preview;
}
