use super::*;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

fn base(schema: &str, summary: &SellbackSummary) -> Value {
    json!({
        "schema":schema, "asset_id":"esk", "symbol":"ESK", "decimals":6,
        "source":"platform_recorded", "chain_status":"not_deployed",
        "simulated":false, "funds_moved":false,
        "verification_basis":"authenticated_operator_review",
        "external_payment_verified":false, "sellback_settlement":false,
        "summary":summary_value(summary),
    })
}

fn summary_value(summary: &SellbackSummary) -> Value {
    json!({
        "snapshot_digest":summary.snapshot_digest,
        "total_base_units":summary.total_base_units.to_string(),
        "reserved_base_units":summary.reserved_base_units.to_string(),
        "available_base_units":summary.available_base_units.to_string(),
        "open_request_count":summary.open_request_count.to_string(),
        "request_count":summary.request_count.to_string(),
        "new_requests_enabled":summary.availability.new_requests_enabled,
        "unavailable_reason":summary.availability.reason,
        "policy":summary.availability.policy.as_ref().map(public_policy),
    })
}

/// Deliberately project only this user's public terms, never the configured allowlist or global cap.
fn public_policy(policy: &SellbackPolicy) -> Value {
    let body = &policy.body;
    json!({
        "policy_digest":policy.policy_digest, "revision":body.revision,
        "terms_digest":body.terms_digest, "terms_text":body.terms_text,
        "min_request_base_units":body.min_request_base_units,
        "max_request_base_units":body.max_request_base_units,
        "max_open_requests_per_user":body.max_open_requests_per_user,
        "max_reserved_base_units_per_user":body.max_reserved_base_units_per_user,
        "hold_mode":body.hold_mode, "cancel_mode":body.cancel_mode,
        "expiry_mode":body.expiry_mode, "participation_effect":body.participation_effect,
        "disabled_account_recovery_text":body.disabled_account_recovery_text,
    })
}

fn record_value(record: &SellbackRecord) -> Value {
    json!({
        "request_id":record.request_id, "idempotency_key":record.input.idempotency_key,
        "amount_base_units":record.input.amount_base_units.to_string(),
        "expected_snapshot_digest":record.input.expected_snapshot_digest,
        "request_digest":record.request_digest,
        "policy_revision":record.policy.body.revision, "policy_digest":record.policy.policy_digest,
        "terms_digest":record.policy.body.terms_digest, "created_at":record.created_at,
        "canceled_at":record.canceled_at, "cancel_event_id":record.cancel_event_id,
        "status":if record.canceled_at.is_some() { "canceled" } else { "submitted" },
    })
}

pub(super) fn page_response(page: SellbackPage) -> Response {
    let mut body = base("yilong.esk.platform_sellback_page.v1", &page.summary);
    body["requests"] = page.requests.iter().map(record_value).collect();
    body["range_start"] = page.range_start.to_string().into();
    body["range_end"] = page.range_end.to_string().into();
    body["has_more"] = page.has_more.into();
    body["next_cursor"] = json!(page.next_cursor);
    Json(body).into_response()
}

pub(super) fn result_response(result: SellbackResult) -> Response {
    let mut body = base("yilong.esk.platform_sellback_result.v1", &result.summary);
    body["request"] = record_value(&result.request);
    body["replayed"] = result.replayed.into();
    Json(body).into_response()
}
