//! Compute-inert endpoint-authentication WebSocket.
//!
//! This route can establish a durable endpoint session binding, but it deliberately exposes no
//! AgentManager task entry, NodeRegistry presence, capability update, command, plugin, or ACK path.

mod handler;
mod rate_limit;

pub(in crate::node_endpoint_transport) use handler::session_ws;
