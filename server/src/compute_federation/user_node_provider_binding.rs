//! Immutable identity binding between one opted-in node installation and one `user_node`
//! Provider.
//!
//! This Domain records source facts only. It does not prove current consent, endpoint reachability,
//! technical readiness, routing, dispatch, execution, market eligibility, or settlement authority.

mod canonical;
mod types;
mod validated;

pub(crate) use canonical::{
    canonical_user_node_provider_binding_receipt_json_and_digest,
    canonical_user_node_provider_binding_request_digest,
};
pub(crate) use types::*;
pub(crate) use validated::{
    build_user_node_provider_binding_receipt, user_node_provider_binding_json_is_canonical,
    user_node_provider_binding_receipt_from_json, validate_user_node_provider_binding_receipt,
};
