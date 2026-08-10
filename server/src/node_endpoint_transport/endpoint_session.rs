//! Endpoint-authentication WebSocket with one narrowly fenced planning-bootstrap profile.
//!
//! V1 remains authentication-only. V2 may exchange only the six-message sharing -> preparation ->
//! planning bootstrap chain and still exposes no task, Ready, route, lease, or execution path.

mod handler;
mod planning;
mod rate_limit;

pub(in crate::node_endpoint_transport) use handler::session_ws;

const MAX_REGISTER_BYTES: usize = 64 * 1024;
const MAX_FRAME_BYTES: usize = 64 * 1024;
const MAX_MESSAGE_BYTES: usize = 576 * 1024;

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum EndpointSessionRegister {
    V1(homecli_proto::NodeEndpointSessionRegisterV1),
    V2(homecli_proto::NodeEndpointSessionRegisterV2),
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum EndpointSessionProtocol {
    AuthenticationOnlyV13,
    PlanningBootstrapV14,
}
