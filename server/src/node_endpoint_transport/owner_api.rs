//! Direct-TLS-only owner API for atomic endpoint credential mutations.
//!
//! These routes are mounted only on the dedicated rustls listener. They never appear on the
//! legacy HTTP router and never accept OWNER_TOKEN, query-string credentials, or proxy headers.

mod contracts;
mod handlers;
mod rate_limit;
mod response;

pub(super) use handlers::{issue, recover, revoke, rotate};
