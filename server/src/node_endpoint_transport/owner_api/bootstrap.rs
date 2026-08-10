//! Direct-TLS-only owner bootstrap endpoints.
//!
//! Bootstrap creates an ordinary database bearer and manages only the legacy node anchor. It
//! never issues endpoint authority, upgrades a socket, or advertises an endpoint origin.

mod contracts;
mod handlers;

pub(super) use handlers::{login, register_node};
