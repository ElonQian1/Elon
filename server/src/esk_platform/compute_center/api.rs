use super::ReferenceQuote;
use crate::{
    esk_asset::platform::{api::real_user, sellback},
    project_auth::json_error,
    types::AppState,
};
use axum::{
    extract::{rejection::QueryRejection, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CenterQuery {
    #[serde(default = "first_page")]
    page: usize,
}
fn first_page() -> usize {
    1
}

pub(super) async fn get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<CenterQuery>, QueryRejection>,
) -> Response {
    let (user, token) = match real_user(&state, &headers) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let query = match query {
        Ok(Query(value)) if (1..=1000).contains(&value.page) => value,
        _ => return json_error(StatusCode::BAD_REQUEST, "无效的账单页码"),
    };
    let snapshot = match state.store.esk_compute_center(
        &user.id,
        token,
        query.page,
        &sellback::load_configuration(),
    ) {
        Ok(value) => value,
        Err(_) => {
            return json_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "账户中心暂不可用，请重新确认登录后重试",
            );
        }
    };
    let at = chrono::Utc::now().timestamp_millis();
    let configured = match std::env::var("ESK_COMPUTE_REFERENCE_QUOTE_JSON") {
        Ok(value) => Some(value),
        Err(_) => match state
            .store
            .billing_get_config("esk_compute_reference_quote")
        {
            Ok(value) => value,
            Err(_) => {
                return json_error(StatusCode::SERVICE_UNAVAILABLE, "参考报价读取失败，请重试")
            }
        },
    };
    let (quote, quote_status) = reference_quote(at, configured);
    let valuation = quote.as_ref().and_then(|value| {
        super::natural(&snapshot.asset.total_base_units)
            .ok()
            .and_then(|units| value.value(units, at).ok())
    });
    let quote_status = if quote.is_some() && valuation.is_none() {
        "valuation_unavailable"
    } else {
        quote_status
    };
    Json(json!({
        "schema":"yilong.esk.compute_center.v1", "observed_at_ms":at,
        "fresh_until_ms":at + 60_000, "asset":snapshot.asset,
        "asset_source":"platform_recorded", "chain_status":"not_deployed",
        "simulated":false, "funds_moved":false,
        "quote":quote, "quote_status":quote_status, "valuation":valuation,
        "valuation_basis":"reference_only", "month_basis":"UTC_calendar_month",
        "usage_sources":snapshot.usage_sources,
        "billing":{"currency":"CNY", "balance_fen":snapshot.legacy_balance_fen,
            "month_cost_fen":snapshot.month_cost_fen, "bills":snapshot.bills,
            "page":query.page, "has_more":snapshot.bills_has_more,
            "holds":snapshot.holds,"holds_has_more":snapshot.holds_has_more},
        "capabilities":{"purchase":false,"esk_service_spending":false},
        "esk_service_reserved_base_units":null,"esk_service_spent_base_units":null,
        "payment_status":"collection_channel_not_configured",
        "service_status":"esk_settlement_not_connected"
    }))
    .into_response()
}

fn reference_quote(at: i64, configured: Option<String>) -> (Option<ReferenceQuote>, &'static str) {
    let Some(raw) = configured else {
        return (None, "not_configured");
    };
    if raw.len() > 2048 {
        return (None, "invalid");
    }
    let Ok(quote) = serde_json::from_str::<ReferenceQuote>(&raw) else {
        return (None, "invalid");
    };
    if quote.validate(at).is_err() {
        return (None, "stale_or_invalid");
    }
    (Some(quote), "fresh")
}
