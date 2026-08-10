//! Direct-TLS-only owner API for endpoint credential mutations and gated legacy bootstrap.
//!
//! These handlers are mounted only on the dedicated rustls listener. Bootstrap deliberately
//! reuses public path strings without reusing legacy extractors, and never accepts OWNER_TOKEN,
//! query-string credentials, or proxy headers.

mod bootstrap;
mod contracts;
mod handlers;
mod ingress;
mod rate_limit;
mod response;

pub(super) use bootstrap::{login as bootstrap_login, register_node as bootstrap_register_node};
pub(super) use handlers::{issue, recover, revoke, rotate};
