use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::project_auth::json_error;

use super::{AddressBindingChallenge, AddressBindingError, AddressBindingRecord};

const BINDING_SCHEMA: &str = "yilong.esk.sui.platform_address_binding.v2";
const INVALID_INPUT_CODE: &str = "ESK_PLATFORM_SUI_BINDING_INVALID_INPUT";
const CONFLICT_CODE: &str = "ESK_PLATFORM_SUI_BINDING_CONFLICT";

pub(super) fn challenge_response(challenge: AddressBindingChallenge) -> Response {
    // The challenge is the exact public V1 wire object. Do not wrap it with
    // user, session, replay, or private persistence metadata.
    Json(challenge).into_response()
}

pub(super) fn bound_response(binding: AddressBindingRecord) -> Response {
    Json(json!({
        "schema": BINDING_SCHEMA,
        "status": "bound",
        "network": binding.network,
        "address": binding.address,
        "signature_scheme": binding.signature_scheme.as_str(),
        "bound_at": binding.bound_at,
        "binding_receipt_sha256": binding.binding_receipt_sha256,
        "address_control_verified": true,
        "platform_subject_authenticated": true,
        "challenge_single_use_recorded": true,
        "chain_finality_verified": false,
        "asset_identity_verified": false,
        "balance_eligible": false,
        "manifest_transition_allowed": false,
    }))
    .into_response()
}

pub(super) fn unbound_response() -> Response {
    Json(json!({
        "schema": BINDING_SCHEMA,
        "status": "unbound",
    }))
    .into_response()
}

pub(super) fn domain_error(error: anyhow::Error) -> Response {
    let Some(kind) = error.downcast_ref::<AddressBindingError>().copied() else {
        tracing::warn!(
            code = "ESK_PLATFORM_SUI_BINDING_STORAGE_ERROR",
            "ESK Sui address binding operation failed"
        );
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ESK_PLATFORM_SUI_BINDING_STORAGE_ERROR",
        );
    };
    let (status, code) = match kind {
        AddressBindingError::InvalidInput
        | AddressBindingError::InvalidChallenge
        | AddressBindingError::InvalidResponse
        | AddressBindingError::ChallengeIdMismatch
        | AddressBindingError::MessageMismatch
        | AddressBindingError::UnsupportedSignatureScheme
        | AddressBindingError::SignatureInvalid => (StatusCode::BAD_REQUEST, INVALID_INPUT_CODE),
        AddressBindingError::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            "ESK_PLATFORM_SUI_BINDING_NOT_AUTHORIZED",
        ),
        AddressBindingError::NotFound => {
            (StatusCode::NOT_FOUND, "ESK_PLATFORM_SUI_BINDING_NOT_FOUND")
        }
        AddressBindingError::RateLimited => (
            StatusCode::TOO_MANY_REQUESTS,
            "ESK_PLATFORM_SUI_BINDING_RATE_LIMITED",
        ),
        AddressBindingError::NotYetValid
        | AddressBindingError::Expired
        | AddressBindingError::Conflict => (StatusCode::CONFLICT, CONFLICT_CODE),
        AddressBindingError::CorruptLedger => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ESK_PLATFORM_SUI_BINDING_LEDGER_INCONSISTENT",
        ),
        AddressBindingError::Storage => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "ESK_PLATFORM_SUI_BINDING_STORAGE_ERROR",
        ),
        AddressBindingError::RandomUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "ESK_PLATFORM_SUI_BINDING_RANDOM_UNAVAILABLE",
        ),
    };
    json_error(status, code)
}
