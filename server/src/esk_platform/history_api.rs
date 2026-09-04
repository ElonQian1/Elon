//! Independent history schema; never widen the strict account or Android snapshot contracts.
use std::sync::Arc;

use axum::{
    extract::{rejection::QueryRejection, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{esk_asset::format_esk_amount, types::AppState};

use super::{
    api::{domain_error, real_user},
    PlatformError,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    cursor: Option<String>,
}

fn default_limit() -> usize {
    20
}

pub(super) async fn get_my_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<HistoryQuery>, QueryRejection>,
) -> Response {
    // Keep extraction failures inside the handler so authentication always precedes query errors.
    let (user, token) = match real_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let query = match query {
        Ok(Query(value)) => value,
        Err(_) => return domain_error(PlatformError::InvalidInput.into()),
    };
    if !(1..=100).contains(&query.limit) {
        return domain_error(PlatformError::InvalidInput.into());
    }
    // The production Store revalidates this real session within the same history read transaction.
    match state
        .store
        .esk_platform_history(&user.id, token, query.limit, query.cursor.as_deref())
    {
        Ok(page) => Json(json!({
            "schema": "yilong.esk.platform_history.v1",
            "asset_id": "esk", "symbol": "ESK", "decimals": 6,
            "source": "platform_recorded", "chain_status": "not_deployed",
            "simulated": false, "funds_moved": false,
            "verification_basis": "authenticated_operator_review",
            "external_payment_verified": false,
            "snapshot_digest": page.snapshot_digest,
            "total": format_esk_amount(page.total_base_units),
            "total_base_units": page.total_base_units.to_string(),
            "entry_count": page.entry_count.to_string(),
            "range_start": page.range_start.to_string(),
            "range_end": page.range_end.to_string(),
            "updated_at": page.updated_at,
            "entries": page.entries.into_iter().map(|entry| json!({
                "entry_id": entry.entry_id,
                "allocation_id": entry.allocation_id,
                "amount": format_esk_amount(entry.amount_base_units),
                "amount_base_units": entry.amount_base_units.to_string(),
                "created_at": entry.created_at,
                "kind": "approved_payment_allocation",
            })).collect::<Vec<_>>(),
            "has_more": page.has_more,
            "next_cursor": page.next_cursor,
        }))
        .into_response(),
        Err(error) => domain_error(error),
    }
}
