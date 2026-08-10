use axum::{extract::Extension, http::StatusCode, routing::get, Router};

use super::evidence_slot::VerifiedSecureTransportSlot;

pub(super) fn build(slot: VerifiedSecureTransportSlot) -> Router {
    Router::new()
        .route("/agent/ws", get(reject_unwired_endpoint_session))
        .layer(Extension(slot))
}

async fn reject_unwired_endpoint_session(
    Extension(slot): Extension<VerifiedSecureTransportSlot>,
) -> (StatusCode, &'static str) {
    // Consume and immediately drop the single-use proof. Until credential issuance and the v216
    // Store/WS bridge land together, this listener must never upgrade or reach legacy auth.
    let _ = slot.take();
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "NODE_ENDPOINT_CREDENTIAL_BRIDGE_UNWIRED",
    )
}
