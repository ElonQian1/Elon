use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    admin::check_auth,
    project_auth::json_error,
    store::{RealtimeCloseMetricRow, Store},
    types::AppState,
};

use super::{close_metric_snapshot, realtime_diagnostics_catalog};

pub async fn admin_close_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    Json(admin_close_metrics_payload(
        &state.store,
        chrono::Utc::now().timestamp(),
    ))
    .into_response()
}

pub async fn admin_diagnostics(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !check_auth(&headers, &state.admin_token) {
        return json_error(StatusCode::UNAUTHORIZED, "无效的管理员令牌");
    }

    Json(realtime_diagnostics_catalog()).into_response()
}

pub(crate) fn admin_close_metrics_payload(store: &Store, now_unix: i64) -> serde_json::Value {
    let all_time = query_store_window(store, None, "all_time");
    let last_1h = query_store_window(store, Some(now_unix - 60 * 60), "last_1h");
    let last_24h = query_store_window(store, Some(now_unix - 24 * 60 * 60), "last_24h");
    let process = close_metric_snapshot();
    let alerts = match store.refresh_realtime_close_alerts() {
        Ok(alerts) => alerts,
        Err(error) => {
            tracing::warn!(
                target: "realtime_metrics",
                error = %error,
                "failed to refresh realtime close alerts"
            );
            Vec::new()
        }
    };

    serde_json::json!({
        "metrics": all_time,
        "alerts": alerts,
        "windows": {
            "all_time": all_time,
            "last_1h": last_1h,
            "last_24h": last_24h,
            "process": process,
        }
    })
}

fn query_store_window(
    store: &Store,
    since_unix: Option<i64>,
    window_name: &str,
) -> Vec<RealtimeCloseMetricRow> {
    match store.admin_realtime_close_metrics_since(since_unix) {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(
                target: "realtime_metrics",
                window = window_name,
                error = %error,
                "failed to load realtime close metrics"
            );
            Vec::new()
        }
    }
}
