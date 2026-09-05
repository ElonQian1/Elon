use axum::{
    body::Bytes,
    extract::RawQuery,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::{proxy, PublicQuantEndpoint};

pub(super) async fn handle(method: Method, RawQuery(query): RawQuery, _body: Bytes) -> Response {
    if method != Method::GET {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    if query.is_some() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    proxy(PublicQuantEndpoint::TeamAssets, None, None).await
}

// The public boundary rejects future/raw account fields. Business valuation remains in quant.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicSummary {
    schema: String,
    source: String,
    scope: String,
    valuation_method: String,
    quote_asset: String,
    status: String,
    total_estimated_value: Option<String>,
    last_success_at_ms: Option<u64>,
    fresh_until_ms: Option<u64>,
    read_only: bool,
    holdings_disclosed: bool,
    funds_moved: bool,
}

pub(super) fn public_body(bytes: &[u8], status: StatusCode) -> Option<Bytes> {
    if status != StatusCode::OK || bytes.len() > 8192 {
        return None;
    }
    let summary: PublicSummary = serde_json::from_slice(bytes).ok()?;
    if summary.schema != "yilong.quant.team_assets.summary.v1"
        || summary.source != "binance"
        || summary.scope != "spot"
        || summary.valuation_method != "binance_wallet_balance"
        || summary.quote_asset != "BTC"
        || !summary.read_only
        || summary.holdings_disclosed
        || summary.funds_moved
    {
        return None;
    }
    match summary.status.as_str() {
        "fresh" | "stale" => {
            let amount = summary.total_estimated_value.as_deref()?;
            let (whole, fraction) = amount.split_once('.').unwrap_or((amount, ""));
            if whole.is_empty()
                || whole.len() > 18
                || !whole.bytes().all(|b| b.is_ascii_digit())
                || (whole.len() > 1 && whole.starts_with('0'))
                || fraction.len() > 8
                || !fraction.bytes().all(|b| b.is_ascii_digit())
                || amount.ends_with('.')
                || summary.last_success_at_ms? == 0
                || summary.fresh_until_ms? == 0
            {
                return None;
            }
        }
        "unavailable" => {
            if summary.total_estimated_value.is_some()
                || summary.last_success_at_ms.is_some()
                || summary.fresh_until_ms.is_some()
            {
                return None;
            }
        }
        _ => return None,
    }
    // Require explicit nulls instead of silently accepting omitted public contract fields.
    let object: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    (object.as_object()?.len() == 12).then(|| Bytes::copy_from_slice(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, Router};
    use serde_json::json;
    use std::path::Path;
    use tower::ServiceExt;

    fn summary() -> serde_json::Value {
        json!({
            "schema":"yilong.quant.team_assets.summary.v1", "source":"binance",
            "scope":"spot", "valuation_method":"binance_wallet_balance", "quote_asset":"BTC",
            "status":"fresh", "total_estimated_value":"1.23456789",
            "last_success_at_ms":1788616000000_u64, "fresh_until_ms":1788616180000_u64,
            "read_only":true, "holdings_disclosed":false, "funds_moved":false
        })
    }

    #[test]
    fn only_approved_summary_fields_cross_public_boundary() {
        let body = summary().to_string();
        assert_eq!(public_body(body.as_bytes(), StatusCode::OK).unwrap(), body);
        for forbidden in ["uid", "apiKey", "balances", "assetBalances", "wallets"] {
            let mut value = summary();
            value[forbidden] = json!("SYNTHETIC_PRIVATE_CANARY");
            assert!(public_body(value.to_string().as_bytes(), StatusCode::OK).is_none());
        }
        let duplicate = body.replacen('{', "{\"quote_asset\":\"BTC\",", 1);
        assert!(public_body(duplicate.as_bytes(), StatusCode::OK).is_none());
        assert!(public_body(body.as_bytes(), StatusCode::INTERNAL_SERVER_ERROR).is_none());
        assert!(public_body(&vec![b' '; 8193], StatusCode::OK).is_none());
    }

    #[test]
    fn unavailable_never_leaks_a_hidden_value_or_wrong_asset_contract() {
        let mut value = summary();
        value["status"] = json!("unavailable");
        assert!(public_body(value.to_string().as_bytes(), StatusCode::OK).is_none());
        for field in [
            "total_estimated_value",
            "last_success_at_ms",
            "fresh_until_ms",
        ] {
            value[field] = serde_json::Value::Null;
        }
        assert!(public_body(value.to_string().as_bytes(), StatusCode::OK).is_some());
        value.as_object_mut().unwrap().remove("last_success_at_ms");
        assert!(public_body(value.to_string().as_bytes(), StatusCode::OK).is_none());
        for amount in ["0.00000000", "999999999999999999.99999999"] {
            let mut value = summary();
            value["total_estimated_value"] = json!(amount);
            assert!(public_body(value.to_string().as_bytes(), StatusCode::OK).is_some());
        }
        for amount in ["-1", "1e8", "01", "1.", "0.000000001", "secret-canary"] {
            let mut value = summary();
            value["total_estimated_value"] = json!(amount);
            assert!(public_body(value.to_string().as_bytes(), StatusCode::OK).is_none());
        }
    }

    #[tokio::test]
    async fn team_summary_rejects_parameters_methods_and_body_before_upstream() {
        let app: Router = super::super::routes(Path::new("missing-quant-preview-dist"));
        for query in ["?account=private", "?url=https://example.invalid", "?"] {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!("/quant/api/v1/team-assets/summary{query}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        for method in [Method::HEAD, Method::POST, Method::PUT, Method::DELETE] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/quant/api/v1/team-assets/summary")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        }
        let response = app
            .clone()
            .oneshot(
                Request::get("/quant/api/v1/team-assets/summary")
                    .body(Body::from("forbidden"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let response = app
            .oneshot(
                Request::get("/quant/api/v1/team-assets/summary/private")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
