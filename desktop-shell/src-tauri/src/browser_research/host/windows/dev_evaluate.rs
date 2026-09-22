//! Development-only page expression evaluation on an observed research host.
//! The caller (`dev_eval`) enforces the switch; this module only performs one bounded CDP call.
use super::HostHandle;
use super::{cdp, CAPTURES};
use serde_json::{json, Value};
use std::{sync::mpsc, time::Duration};

const RESULT_LIMIT: usize = 16 * 1024;
const WAIT: Duration = Duration::from_secs(8);

pub(in crate::browser_research::host) fn evaluate(
    app: &tauri::AppHandle,
    handle: &HostHandle,
    expression: String,
) -> Result<Value, String> {
    if !handle.active() {
        return Err("browser_research_session_inactive".into());
    }
    let (tx, rx) = mpsc::sync_channel::<Result<Value, &'static str>>(1);
    let label = handle.label.clone();
    let generation = handle.generation();
    let dispatched = app.run_on_main_thread(move || {
        let outcome = CAPTURES.with(|states| {
            let Some(context) = states.borrow().get(&label).cloned() else {
                return Err("research_host_unavailable");
            };
            {
                let state = context.borrow();
                if !state.ready
                    || state.handle.generation() != generation
                    || !state.config.allows_document(&state.document_url)
                {
                    return Err("host_document_not_observed");
                }
            }
            let sender = tx.clone();
            let ok = cdp::call(
                &context,
                cdp::Method::Evaluate,
                json!({"expression":expression,"returnByValue":true,"awaitPromise":true,
                    "timeout":5000,"userGesture":false}),
                move |_, result| {
                    let _ = sender.try_send(result.map_err(|_| "evaluate_failed"));
                },
            );
            if ok {
                Ok(())
            } else {
                Err("host_cdp_dispatch_failed")
            }
        });
        if let Err(code) = outcome {
            let _ = tx.try_send(Err(code));
        }
    });
    if dispatched.is_err() {
        return Err("browser_research_host_dispatch_failed".into());
    }
    let raw = rx
        .recv_timeout(WAIT)
        .map_err(|_| "evaluate_timed_out".to_string())??;
    Ok(bounded(raw))
}

/// Keep only the value-by-value result and exception text; description/preview fields can echo
/// large page state and are dropped before the credential filter sees the text.
fn bounded(raw: Value) -> Value {
    let value = raw.pointer("/result/value").cloned().unwrap_or(Value::Null);
    let kind = raw
        .pointer("/result/type")
        .and_then(Value::as_str)
        .unwrap_or("undefined")
        .to_string();
    let exception = raw
        .pointer("/exceptionDetails/exception/description")
        .or_else(|| raw.pointer("/exceptionDetails/text"))
        .and_then(Value::as_str)
        .map(|text| text.chars().take(512).collect::<String>());
    let mut text = serde_json::to_string(&value).unwrap_or_default();
    let truncated = text.len() > RESULT_LIMIT;
    if truncated {
        let mut end = RESULT_LIMIT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    json!({"type":kind,"value_json":text,"truncated":truncated,"exception":exception})
}
