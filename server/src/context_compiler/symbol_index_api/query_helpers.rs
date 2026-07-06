use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::Value;

pub(super) fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn parse_must_include_value(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => split_must_include(Some(text)),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str())
            .flat_map(|text| split_must_include(Some(text)))
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn split_must_include(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(|ch| ch == ',' || ch == ';' || ch == '\n' || ch == '\r')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn json_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": message,
        })),
    )
        .into_response()
}
