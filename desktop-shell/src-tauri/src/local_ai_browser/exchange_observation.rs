//! Latest adapter-validated observations per exchange session webview, kept apart from the
//! chat-oriented `LocalAiBrowserRuntime` record so exchange facts never masquerade as chat state.
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, State, Webview};

use super::{
    adapter::SanitizedAdapterEvent, adapter_command, ensure_main_webview, ensure_session_webview,
    provider_for_kind, resolve_owner_fingerprint, window_label, ProviderKind,
};

const SCHEMA: &str = "yilong.exchange_webview.observation.v1";
const MAX_DETAILS: usize = 64;
const MAX_REPORTS: usize = 16;
const MAX_COMMAND_RESULTS: usize = 8;

#[derive(Clone, Default)]
pub struct ExchangeObservationRuntime {
    sessions: Arc<Mutex<HashMap<String, ExchangeObservation>>>,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeObservation {
    schema: &'static str,
    adapter_ready: bool,
    document_token: Option<String>,
    identity: Option<Value>,
    list: Option<Value>,
    details: BTreeMap<String, Value>,
    reports: BTreeMap<String, Value>,
    wallet: Option<Value>,
    diagnostic: Option<Value>,
    unavailable_at_ms: u64,
    command_results: Vec<Value>,
    last_error: Option<String>,
    updated_at_ms: u64,
}

impl ExchangeObservationRuntime {
    fn sessions(&self) -> MutexGuard<'_, HashMap<String, ExchangeObservation>> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Apply one sanitized adapter event. A new document token replaces every earlier fact,
    /// because a reload or account switch makes the previous page's observations stale.
    pub(super) fn record(&self, label: &str, event: &SanitizedAdapterEvent) {
        let mut sessions = self.sessions();
        let record = sessions.entry(label.to_string()).or_default();
        record.schema = SCHEMA;
        if let Some(token) = event.document_token.as_deref() {
            if record.document_token.as_deref() != Some(token) {
                *record = ExchangeObservation {
                    schema: SCHEMA,
                    document_token: Some(token.to_string()),
                    ..ExchangeObservation::default()
                };
            }
        }
        match event.kind.as_str() {
            "adapter_ready" => {
                record.adapter_ready = true;
                record.last_error = None;
            }
            "exchange_observation" => apply_observation(record, &event.payload),
            "command_result" => {
                record.command_results.push(event.payload.clone());
                if record.command_results.len() > MAX_COMMAND_RESULTS {
                    record.command_results.remove(0);
                }
            }
            "browser_diagnostic" => {
                record.last_error = event
                    .payload
                    .get("detail")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            _ => {}
        }
        record.updated_at_ms = now_ms();
    }

    fn snapshot(&self, label: &str, window_open: bool) -> Value {
        let record = self.sessions().get(label).cloned().unwrap_or_default();
        let mut value = serde_json::to_value(record).unwrap_or_else(|_| json!({}));
        value["schema"] = json!(SCHEMA);
        value["windowOpen"] = json!(window_open);
        value["tradingEnabled"] = json!(false);
        value
    }

    pub(super) fn forget(&self, label: &str) {
        self.sessions().remove(label);
    }
}

fn apply_observation(record: &mut ExchangeObservation, payload: &Value) {
    let schema = payload.get("schema").and_then(Value::as_str).unwrap_or("");
    let kind = payload.get("kind").and_then(Value::as_str).unwrap_or("");
    match schema {
        "yilong.binance_observation.v1" => match kind {
            "identity" => record.identity = Some(payload.clone()),
            "list" => {
                record.list = Some(payload.clone());
                record.details.clear();
                record.unavailable_at_ms = 0;
            }
            "detail" => {
                let id = payload
                    .pointer("/row/id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if !id.is_empty() {
                    if record.details.len() >= MAX_DETAILS && !record.details.contains_key(&id) {
                        record.details.clear();
                    }
                    record.details.insert(id, payload.clone());
                }
            }
            "unavailable" => {
                record.list = None;
                record.details.clear();
                record.unavailable_at_ms = now_ms();
            }
            _ => {}
        },
        "yilong.binance_report_observation.v1" => {
            let request = payload
                .get("request")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if !request.is_empty() {
                if record.reports.len() >= MAX_REPORTS && !record.reports.contains_key(&request) {
                    record.reports.clear();
                }
                record.reports.insert(request, payload.clone());
            }
        }
        "yilong.binance_wallet_observation.v1" => record.wallet = Some(payload.clone()),
        "yilong.binance_diagnostic.v1" => record.diagnostic = Some(payload.clone()),
        _ => {}
    }
}

#[tauri::command]
pub(crate) fn get_exchange_web_observation(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, ExchangeObservationRuntime>,
    provider_id: String,
    owner_key: String,
) -> Result<Value, String> {
    ensure_main_webview(&webview)?;
    let provider = provider_for_kind(&provider_id, ProviderKind::Exchange)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    let open = app.get_webview(&label).is_some();
    if !open {
        runtime.forget(&label);
    }
    Ok(runtime.snapshot(&label, open))
}

/// Drive the read-only adapter surface (`refresh`, `detail`, `report`, `wallet`, `inspect`).
/// There is no create, manage or trading action on the Win host.
#[tauri::command]
pub(crate) fn run_exchange_web_adapter_command(
    app: AppHandle,
    webview: Webview,
    provider_id: String,
    owner_key: String,
    action: String,
    value: Option<String>,
    request_id: Option<String>,
) -> Result<(), String> {
    ensure_main_webview(&webview)?;
    let provider = provider_for_kind(&provider_id, ProviderKind::Exchange)?;
    let adapter = provider
        .adapter
        .ok_or_else(|| format!("{} 当前没有本机读取适配器。", provider.display_name))?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    let page = app
        .get_webview(&label)
        .ok_or_else(|| format!("请先打开 {} 官网窗口。", provider.display_name))?;
    let command = adapter_command::build(
        provider.display_name,
        adapter.supported_actions(),
        &action,
        value,
        None,
        request_id,
    )?;
    let raw = serde_json::to_string(&command).map_err(|error| error.to_string())?;
    page.eval(adapter.page_invocation_script(&raw)?)
        .map_err(|error| error.to_string())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(kind: &str, extra: Value, token: &str) -> SanitizedAdapterEvent {
        let mut payload = json!({"schema":"yilong.binance_observation.v1","kind":kind});
        if let (Some(target), Some(source)) = (payload.as_object_mut(), extra.as_object()) {
            for (key, value) in source {
                target.insert(key.clone(), value.clone());
            }
        }
        SanitizedAdapterEvent {
            kind: "exchange_observation".into(),
            payload,
            page_context_key: None,
            restorable_url: None,
            document_token: Some(token.into()),
        }
    }

    #[test]
    fn list_details_and_unavailable_follow_the_adapter_lifecycle() {
        let runtime = ExchangeObservationRuntime::default();
        runtime.record("w", &observation("identity", json!({"account":"1"}), "doc_a"));
        runtime.record("w", &observation("list", json!({"rows":[{"id":"7"}]}), "doc_a"));
        runtime.record("w", &observation("detail", json!({"row":{"id":"7"}}), "doc_a"));
        let snapshot = runtime.snapshot("w", true);
        assert_eq!(snapshot["identity"]["account"], "1");
        assert_eq!(snapshot["list"]["rows"][0]["id"], "7");
        assert!(snapshot["details"]["7"].is_object());
        assert_eq!(snapshot["tradingEnabled"], false);
        runtime.record("w", &observation("unavailable", json!({}), "doc_a"));
        let snapshot = runtime.snapshot("w", true);
        assert!(snapshot["list"].is_null());
        assert!(snapshot["details"].as_object().unwrap().is_empty());
        assert!(snapshot["unavailableAtMs"].as_u64().unwrap() > 0);
    }

    #[test]
    fn a_new_document_token_discards_the_previous_pages_facts() {
        let runtime = ExchangeObservationRuntime::default();
        runtime.record("w", &observation("list", json!({"rows":[]}), "doc_a"));
        runtime.record(
            "w",
            &SanitizedAdapterEvent {
                kind: "adapter_ready".into(),
                payload: json!({"type":"adapter_ready"}),
                page_context_key: None,
                restorable_url: None,
                document_token: Some("doc_b".into()),
            },
        );
        let snapshot = runtime.snapshot("w", true);
        assert!(snapshot["list"].is_null());
        assert_eq!(snapshot["adapterReady"], true);
        assert_eq!(snapshot["documentToken"], "doc_b");
    }

    #[test]
    fn reports_and_command_results_stay_bounded() {
        let runtime = ExchangeObservationRuntime::default();
        for index in 0..(MAX_REPORTS + 2) {
            let mut event = observation("matches", json!({"request":format!("r{index}")}), "doc_a");
            event.payload["schema"] = json!("yilong.binance_report_observation.v1");
            runtime.record("w", &event);
        }
        for index in 0..(MAX_COMMAND_RESULTS + 3) {
            runtime.record(
                "w",
                &SanitizedAdapterEvent {
                    kind: "command_result".into(),
                    payload: json!({"action":"refresh","ok":true,"index":index}),
                    page_context_key: None,
                    restorable_url: None,
                    document_token: None,
                },
            );
        }
        let snapshot = runtime.snapshot("w", true);
        assert!(snapshot["reports"].as_object().unwrap().len() <= MAX_REPORTS);
        assert_eq!(
            snapshot["commandResults"].as_array().unwrap().len(),
            MAX_COMMAND_RESULTS
        );
        assert_eq!(snapshot["windowOpen"], true);
    }
}
