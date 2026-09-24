//! Fixed, read-only ChatGPT operation. No caller-supplied JavaScript or URL.
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, State, Webview};

use super::{provider, resolve_owner_fingerprint, window_label, LocalAiBrowserRuntime};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    conversation_id: String,
    request_id: String,
    #[serde(default)]
    cursor: String,
}

pub(crate) fn validate(query: &str) -> Result<(), String> {
    let request: Request =
        serde_json::from_str(query).map_err(|_| "invalid_conversation_request")?;
    let id = &request.conversation_id;
    if id.len() != 36
        || !id.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
        || !(8..=80).contains(&request.request_id.len())
        || !request
            .request_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        || request.cursor.len() > 100
        || !request
            .cursor
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.')
    {
        return Err("invalid_conversation_request".into());
    }
    Ok(())
}

pub(crate) async fn read(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, LocalAiBrowserRuntime>,
    owner: String,
    query: String,
) -> Result<Value, String> {
    validate(&query)?;
    let input: Request =
        serde_json::from_str(&query).map_err(|_| "invalid_conversation_request")?;
    let provider = provider("chatgpt")?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner)?;
    super::ensure_session_webview(&webview, provider, &fingerprint)?;
    let label = window_label(provider, &fingerprint);
    if app.get_webview(&label).is_none() {
        super::open_local_ai_web_session(
            app.clone(),
            webview,
            runtime,
            "chatgpt".into(),
            owner,
            Some(false),
        )
        .await?;
        return Ok(
            json!({"schema":"yilong.browser-research.result.v1", "kind":"read_conversation",
            "reader":{"status":"pending","request_id":input.request_id}}),
        );
    }
    let page = app.get_webview(&label).ok_or("reader_unavailable")?;
    let expected_url = page.url().map_err(|_| "reader_unavailable")?;
    if expected_url.origin().ascii_serialization() != "https://chatgpt.com" {
        let pending = expected_url.as_str() == "about:blank";
        if !pending {
            super::embedded_view::restore_popout(&app, &label)?;
            runtime.mark_window_visible(&label, true);
        }
        return Ok(
            json!({"schema":"yilong.browser-research.result.v1", "kind":"read_conversation",
            "reader":if pending { json!({"status":"pending","request_id":input.request_id}) }
                else { json!({"status":"failed","request_id":input.request_id,"error":"login_required"}) }}),
        );
    }
    let result = execute(page.clone(), query).await.unwrap_or_else(|error| {
        json!({"status":"failed","request_id":input.request_id,"error": match error.as_str() {
            "reader_timeout" => "reader_timeout", _ => "reader_unavailable"
        }})
    });
    if page.url().ok().as_ref() != Some(&expected_url) {
        return Err("reader_context_changed".into());
    }
    if matches!(
        result.get("error").and_then(Value::as_str),
        Some("login_required" | "http_401")
    ) {
        // Expose the official login surface; never automate credentials or switch profiles.
        super::embedded_view::restore_popout(&app, &label)?;
        runtime.mark_window_visible(&label, true);
    }
    Ok(
        json!({"schema":"yilong.browser-research.result.v1", "kind":"read_conversation", "reader":result}),
    )
}

#[cfg(windows)]
async fn execute(page: Webview, query: String) -> Result<Value, String> {
    use std::{sync::mpsc, time::Duration};
    use webview2_com::CallDevToolsProtocolMethodCompletedHandler;
    use windows::core::{HSTRING, PCWSTR};
    let projection = include_str!(
        "../../../../android/app/src/main/assets/chatgpt_web_conversation_projection.js"
    );
    let reader =
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_conversation_reader.js");
    let request =
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_json_request.js");
    let auth =
        include_str!("../../../../android/app/src/main/assets/chatgpt_web_private_auth_context.js");
    let expression = format!("(function(){{if(location.origin!=='https://chatgpt.com')return null;\nwindow.__elonChatGptPrivateAuthContextEnabled=true;\n{request}\n{auth}\n{projection}\n{reader}\nreturn window.__elonConversationReader.run({query});}})()");
    let params =
        json!({"expression":expression,"returnByValue":true,"timeout":3000,"userGesture":false})
            .to_string();
    let (tx, rx) = mpsc::sync_channel(1);
    page.with_webview(move |platform| {
        let sender = tx.clone();
        let callback =
            CallDevToolsProtocolMethodCompletedHandler::create(Box::new(move |status, text| {
                let value = if status.is_ok() && text.len() <= 60 * 1024 {
                    serde_json::from_str::<Value>(&text)
                        .ok()
                        .and_then(|v| v.pointer("/result/value").cloned())
                } else {
                    None
                };
                let _ = sender.try_send(value);
                Ok(())
            }));
        let dispatched = unsafe {
            platform.controller().CoreWebView2().and_then(|core| {
                core.CallDevToolsProtocolMethod(
                    PCWSTR(HSTRING::from("Runtime.evaluate").as_ptr()),
                    PCWSTR(HSTRING::from(params).as_ptr()),
                    &callback,
                )
            })
        };
        if dispatched.is_err() {
            let _ = tx.try_send(None);
        }
    })
    .map_err(|_| "reader_unavailable")?;
    tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(5)))
        .await
        .map_err(|_| "reader_unavailable")?
        .map_err(|_| "reader_timeout")?
        .filter(Value::is_object)
        .ok_or_else(|| "reader_unavailable".into())
}

#[cfg(not(windows))]
async fn execute(_: Webview, _: String) -> Result<Value, String> {
    Err("reader_unsupported".into())
}
