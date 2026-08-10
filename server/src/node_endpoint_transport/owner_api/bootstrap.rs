//! Direct-TLS-only owner bootstrap endpoints.
//!
//! Bootstrap creates an ordinary database bearer and manages only the legacy node anchor. It
//! never issues endpoint authority, upgrades a socket, or advertises an endpoint origin.

mod contracts;
mod handlers;

pub(in crate::node_endpoint_transport) use handlers::{login, register_node};
