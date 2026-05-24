use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::{collections::HashMap, sync::Arc, sync::LazyLock};
use tokio::sync::broadcast;

use crate::types::AppState;

static APP_UPDATE_TX: LazyLock<broadcast::Sender<String>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(64);
    tx
});

pub fn subscribe() -> broadcast::Receiver<String> {
    APP_UPDATE_TX.subscribe()
}

pub async fn latest_update_event_for_client(
    state: &AppState,
    client_version_code: Option<i64>,
) -> Option<String> {
    let event = latest_update_event(state).await.ok()?;
    if is_newer_for_client(&event, client_version_code) {
        Some(event)
    } else {
        None
    }
}

pub fn is_newer_for_client(event: &str, client_version_code: Option<i64>) -> bool {
    let Some(client_version_code) = client_version_code else {
        return true;
    };
    serde_json::from_str::<serde_json::Value>(event)
        .ok()
        .and_then(|json| json.get("versionCode").and_then(|v| v.as_i64()))
        .map(|version_code| version_code > client_version_code)
        .unwrap_or(true)
}

pub async fn broadcast_latest_update(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    if !authorized(&headers, &query) {
        return (StatusCode::UNAUTHORIZED, "invalid update broadcast token").into_response();
    }

    match latest_update_event(&state).await {
        Ok(event) => {
            let receivers = APP_UPDATE_TX.send(event.clone()).unwrap_or(0);
            Json(serde_json::json!({
                "ok": true,
                "receivers": receivers,
                "event": serde_json::from_str::<serde_json::Value>(&event).unwrap_or_default(),
            }))
            .into_response()
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

fn authorized(headers: &HeaderMap, query: &HashMap<String, String>) -> bool {
    let expected = std::env::var("APP_UPDATE_BROADCAST_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let Some(expected) = expected else {
        return true;
    };

    if query
        .get("token")
        .map(|value| value == &expected)
        .unwrap_or(false)
    {
        return true;
    }

    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value == expected)
        .unwrap_or(false)
}

async fn latest_update_event(state: &AppState) -> anyhow::Result<String> {
    let path = state.data_dir.join("app").join("version.json");
    let content = tokio::fs::read_to_string(&path).await?;
    let content = content.trim_start_matches('\u{FEFF}');
    let mut json: serde_json::Value = serde_json::from_str(content)?;
    let public_url = state.public_url.trim_end_matches('/');

    json["type"] = serde_json::Value::String("app_update_available".into());
    json["downloadUrl"] =
        serde_json::Value::String(format!("{public_url}/app/ElonSpeed-latest.apk"));
    json["downloadPageUrl"] = serde_json::Value::String(format!("{public_url}/app/download"));

    Ok(serde_json::to_string(&json)?)
}
