//! Win host for the shared read-only Binance grid adapters. The page keeps every request
//! and credential; native code only accepts the adapters' already validated observations.
use serde_json::{json, Map, Value};

use super::adapter::{self, SanitizedAdapterEvent};

pub(super) const ADAPTER_VERSION: u32 = 1;
const MAX_EVENT_BYTES: usize = 1024 * 1024;
const OBSERVATION_SCHEMAS: &[&str] = &[
    "yilong.binance_observation.v1",
    "yilong.binance_report_observation.v1",
    "yilong.binance_wallet_observation.v1",
    "yilong.binance_diagnostic.v1",
];

const WIN_BRIDGE: &str = include_str!("binance_win_bridge.js");
// Same order as the Android host: diagnostics and factories before the read adapter consumes them.
const ADAPTER_ASSETS: &[(&str, &str)] = &[
    (
        "binance_grid_read_diagnostics.js",
        include_str!("../../../../android/app/src/main/assets/binance_grid_read_diagnostics.js"),
    ),
    (
        "binance_grid_reports_adapter.js",
        include_str!("../../../../android/app/src/main/assets/binance_grid_reports_adapter.js"),
    ),
    (
        "binance_wallet_adapter.js",
        include_str!("../../../../android/app/src/main/assets/binance_wallet_adapter.js"),
    ),
    (
        "binance_grid_read_adapter.js",
        include_str!("../../../../android/app/src/main/assets/binance_grid_read_adapter.js"),
    ),
];

pub(super) fn adapter_asset_names() -> Vec<&'static str> {
    ADAPTER_ASSETS.iter().map(|(name, _)| *name).collect()
}

pub(super) fn initialization_script() -> String {
    let bridge = WIN_BRIDGE.replace("__ADAPTER_VERSION__", &ADAPTER_VERSION.to_string());
    let assets = ADAPTER_ASSETS
        .iter()
        .map(|(name, source)| format!("window.__elonBinanceWinBootstrapStage = '{name}';\n{source}"))
        .collect::<Vec<_>>()
        .join("\n");
    // The bridge defines window.ElonBinanceRead before the adapters and binds the token after them.
    format!(
        "{bridge}\n{assets}\nif (window.__elonBinanceWinBridge) window.__elonBinanceWinBridge.bind();\n"
    )
}

pub(super) fn sanitize_event(raw: &str) -> Result<SanitizedAdapterEvent, String> {
    if raw.len() > MAX_EVENT_BYTES {
        return Err("Binance 观察事件过大，已拒绝。".to_string());
    }
    let value: Value = serde_json::from_str(raw).map_err(|_| "Binance 观察事件格式无效。")?;
    let object = value
        .as_object()
        .ok_or_else(|| "Binance 观察事件必须是对象。".to_string())?;
    if let Some(schema) = object.get("schema").and_then(Value::as_str) {
        return sanitize_observation(schema, object);
    }
    match object.get("type").and_then(Value::as_str) {
        Some("adapter_ready") => {
            let token = object
                .get("token")
                .and_then(Value::as_str)
                .filter(|token| adapter::valid_document_token(token))
                .ok_or_else(|| "Binance 页面文档令牌无效。".to_string())?;
            Ok(event(
                "adapter_ready",
                json!({"type":"adapter_ready"}),
                Some(token.to_string()),
            ))
        }
        Some("command_result") => Ok(event(
            "command_result",
            json!({
                "type": "command_result",
                "action": identifier(object.get("action"), 32),
                "ok": object.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "detail": text(object.get("detail"), 240),
                "requestId": object.get("requestId").and_then(Value::as_str)
                    .filter(|id| id.len() <= 36 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')),
            }),
            None,
        )),
        Some("browser_diagnostic") => Ok(event(
            "browser_diagnostic",
            json!({
                "type": "browser_diagnostic",
                "kind": identifier(object.get("kind"), 48),
                "detail": text(object.get("detail"), 240),
            }),
            None,
        )),
        _ => Err("不支持的 Binance 本地浏览器事件。".to_string()),
    }
}

fn sanitize_observation(
    schema: &str,
    object: &Map<String, Value>,
) -> Result<SanitizedAdapterEvent, String> {
    if !OBSERVATION_SCHEMAS.contains(&schema) {
        return Err("不支持的 Binance 观察事件版本。".to_string());
    }
    let token = object
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| adapter::valid_document_token(token))
        .ok_or_else(|| "Binance 观察事件缺少有效文档令牌。".to_string())?
        .to_string();
    // The adapters emit `kind` for grid/report/wallet events; the diagnostic schema has none.
    let kind = object
        .get("kind")
        .map(|value| identifier(Some(value), 32))
        .unwrap_or_else(|| "diagnostic".to_string());
    if kind.is_empty() {
        return Err("Binance 观察事件类型无效。".to_string());
    }
    let mut payload = object.clone();
    payload.remove("token");
    Ok(event(
        "exchange_observation",
        Value::Object(payload),
        Some(token),
    ))
}

fn event(kind: &str, payload: Value, document_token: Option<String>) -> SanitizedAdapterEvent {
    SanitizedAdapterEvent {
        kind: kind.to_string(),
        payload,
        page_context_key: None,
        restorable_url: None,
        document_token,
    }
}

fn identifier(value: Option<&Value>, max: usize) -> String {
    value
        .and_then(Value::as_str)
        .filter(|text| {
            !text.is_empty()
                && text.len() <= max
                && text
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
        .unwrap_or_default()
        .to_string()
}

fn text(value: Option<&Value>, max: usize) -> String {
    value
        .and_then(Value::as_str)
        .map(|text| text.chars().filter(|c| !c.is_control()).take(max).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_defines_bridge_before_shared_adapters_and_binds_after_them() {
        let script = initialization_script();
        let bridge = script.find("window.ElonBinanceRead = Object.freeze").unwrap();
        let read_adapter = script.find("window.__elonBinanceReadV1 = Object.freeze").unwrap();
        let bind = script.rfind("window.__elonBinanceWinBridge.bind();").unwrap();
        assert!(bridge < read_adapter && read_adapter < bind);
        assert!(script.contains("version: 1,"));
        assert_eq!(
            adapter_asset_names(),
            [
                "binance_grid_read_diagnostics.js",
                "binance_grid_reports_adapter.js",
                "binance_wallet_adapter.js",
                "binance_grid_read_adapter.js"
            ]
        );
        // Write-capable create/manage transports never ship to the Win observer.
        assert!(!script.contains("place-grid"));
    }

    #[test]
    fn observation_events_keep_adapter_payload_and_drop_the_token() {
        let raw = r#"{"schema":"yilong.binance_observation.v1","token":"doc_win_0123abcd","kind":"list","account":"12","account_kind":"sub","rows":[]}"#;
        let event = sanitize_event(raw).unwrap();
        assert_eq!(event.kind, "exchange_observation");
        assert_eq!(event.document_token.as_deref(), Some("doc_win_0123abcd"));
        assert_eq!(event.payload["kind"], "list");
        assert!(event.payload.get("token").is_none());
        let diagnostic = sanitize_event(
            r#"{"schema":"yilong.binance_diagnostic.v1","token":"doc_win_0123abcd","list_requests":1}"#,
        )
        .unwrap();
        assert_eq!(diagnostic.payload["list_requests"], 1);
    }

    #[test]
    fn unknown_schemas_missing_tokens_and_oversized_events_are_rejected() {
        assert!(sanitize_event(r#"{"schema":"yilong.binance_create_event.v1","token":"doc_win_1"}"#).is_err());
        assert!(sanitize_event(r#"{"schema":"yilong.binance_observation.v1","kind":"list"}"#).is_err());
        assert!(sanitize_event(r#"{"schema":"yilong.binance_observation.v1","token":"DOC","kind":"list"}"#).is_err());
        assert!(sanitize_event(r#"{"type":"unknown"}"#).is_err());
        assert!(sanitize_event(&format!(r#"{{"schema":"yilong.binance_observation.v1","token":"doc_win_1","kind":"list","pad":"{}"}}"#, "x".repeat(MAX_EVENT_BYTES))).is_err());
    }

    #[test]
    fn bridge_command_results_and_readiness_are_bounded() {
        let ready = sanitize_event(r#"{"type":"adapter_ready","token":"doc_win_abc"}"#).unwrap();
        assert_eq!(ready.kind, "adapter_ready");
        assert_eq!(ready.document_token.as_deref(), Some("doc_win_abc"));
        assert!(sanitize_event(r#"{"type":"adapter_ready","token":"nope"}"#).is_err());
        let result = sanitize_event(
            r#"{"type":"command_result","action":"refresh","ok":true,"detail":"x\u0000y","requestId":"mcp_1"}"#,
        )
        .unwrap();
        assert_eq!(result.payload["action"], "refresh");
        assert_eq!(result.payload["detail"], "xy");
        assert_eq!(result.payload["requestId"], "mcp_1");
        let odd = sanitize_event(r#"{"type":"command_result","action":"Refresh Now","ok":"yes"}"#).unwrap();
        assert_eq!(odd.payload["action"], "");
        assert_eq!(odd.payload["ok"], false);
    }
}
