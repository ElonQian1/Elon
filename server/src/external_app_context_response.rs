//! HTTP response normalization for external app context fetches.

use serde_json::{json, Value};

use crate::external_app_context_contract::{fb2_match_context, fb2_pack_context};

pub(crate) async fn fb2_pack_response_to_context(
    app_id: &str,
    external_group_id: &str,
    response: reqwest::Response,
) -> Value {
    let status = response.status();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": compact_error(&error.to_string())
            });
        }
    };
    if !status.is_success() {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "status_code": status.as_u16(),
            "error": truncate_chars(&text, 300)
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": format!("fb2 context pack JSON 解析失败：{}", compact_error(&error.to_string()))
            });
        }
    };
    if parsed["success"].as_bool() != Some(true) {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "error": parsed["error"].as_str().unwrap_or("fb2 返回失败状态")
        });
    }

    let data = parsed.get("data").cloned().unwrap_or_else(|| json!({}));
    fb2_pack_context(app_id, external_group_id, data)
}

pub(crate) async fn fb2_response_to_context(
    app_id: &str,
    external_group_id: &str,
    response: reqwest::Response,
) -> Value {
    let status = response.status();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": compact_error(&error.to_string())
            });
        }
    };
    if !status.is_success() {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "status_code": status.as_u16(),
            "error": truncate_chars(&text, 300)
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": format!("fb2 JSON 解析失败：{}", compact_error(&error.to_string()))
            });
        }
    };
    if parsed["success"].as_bool() != Some(true) {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "error": parsed["error"].as_str().unwrap_or("fb2 返回失败状态")
        });
    }

    let data = parsed.get("data").cloned().unwrap_or_else(|| json!({}));
    fb2_match_context(app_id, external_group_id, data)
}

pub(crate) fn compact_error(error: &str) -> String {
    truncate_chars(error, 220)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
#[path = "external_app_context_response_tests.rs"]
mod tests;
