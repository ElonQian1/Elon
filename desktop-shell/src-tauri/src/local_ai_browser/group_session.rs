//! Isolated group-analysis documents share only the account's WebView2 profile.
use super::*;
use serde_json::{json, Value};

pub(super) fn label(
    provider: &ProviderDefinition,
    fingerprint: &str,
    task: &str,
) -> Result<String, String> {
    if task.len() != 36
        || !task.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
    {
        return Err("群聊 AI 操作标识无效。".into());
    }
    Ok(format!(
        "{}-group-{}",
        window_label(provider, fingerprint),
        task.replace('-', "").to_ascii_lowercase()
    ))
}

#[tauri::command]
pub async fn group_ai_web_session(
    app: AppHandle,
    webview: Webview,
    runtime: State<'_, LocalAiBrowserRuntime>,
    owner_key: String,
    provider_id: String,
    task_id: String,
    action: String,
    value: Option<String>,
    request_id: Option<String>,
) -> Result<Value, String> {
    crate::group_ai_worker::ensure_caller(&webview)?;
    if webview.label() != MAIN_WEBVIEW_LABEL && (provider_id != "chatgpt" || action == "show") {
        return Err("group_worker_action_not_allowed".into());
    }
    let provider = provider_for_kind(&provider_id, ProviderKind::AiAssistant)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    let label = label(provider, &fingerprint, &task_id)?;
    if action == "open" {
        return serde_json::to_value(
            session_host::open(
                app,
                webview,
                runtime,
                provider,
                owner_key,
                false,
                Some(&task_id),
            )
            .await?,
        )
        .map_err(display_error);
    }
    if action == "close" {
        if let Some(page) = app.get_webview(&label) {
            page.close().map_err(display_error)?;
        }
        if let Some(window) = app.get_window(&label) {
            window.close().map_err(display_error)?;
        }
        runtime.remove_group_session(&label);
        return Ok(json!({"closed":true}));
    }
    let page = app
        .get_webview(&label)
        .ok_or("群聊 AI 会话已关闭，请重新准备。")?;
    if action == "show" {
        embedded_view::restore_popout(&app, &label)?;
        runtime.mark_window_visible(&label, true);
    } else if action != "state" {
        let (command_action, command_value) = match action.as_str() {
            "prepare" if provider.id == "chatgpt" => (
                "private_protocol_probe",
                Some("fresh_text_admission".into()),
            ),
            "stage_attachments" if provider.id == "chatgpt" => (action.as_str(), value),
            "snapshot" | "send_prompt" | "stop_generation" => (action.as_str(), value),
            _ => return Err("不支持的群聊 AI 动作。".into()),
        };
        if matches!(command_action, "send_prompt" | "stage_attachments") {
            runtime.require_bound_context(&label)?;
            let snapshot = runtime.snapshot(&label).ok_or("群聊 AI 尚未就绪。")?;
            if !ready(
                provider.id,
                page.url().map_err(display_error)?.as_str(),
                snapshot.semantic_event.as_ref(),
            ) {
                return Err("群聊 AI 文档已变化，未发送消息。".into());
            }
        }
        let adapter = provider.adapter.ok_or("网页 AI 适配器不可用。")?;
        let command = adapter_command::build(
            provider.display_name,
            &[
                "snapshot",
                "send_prompt",
                "stop_generation",
                "private_protocol_probe",
                "stage_attachments",
            ],
            command_action,
            command_value.clone(),
            Some(String::new()),
            request_id.clone(),
        )?;
        if action != "snapshot" {
            runtime.mark_command_pending_with_value(
                &label,
                command_action,
                request_id.as_deref(),
                command_value.as_deref(),
            );
        }
        let raw = serde_json::to_string(&command).map_err(display_error)?;
        page.eval(adapter.page_invocation_script(&raw)?)
            .map_err(display_error)?;
        if action == "send_prompt" {
            embedded_view::hide(&app, &label)?;
            runtime.mark_window_visible(&label, false);
        }
    }
    let mut state = serde_json::to_value(runtime.snapshot(&label).ok_or("群聊 AI 状态不可用。")?)
        .map_err(display_error)?;
    // The normal state deliberately strips queries. Expose only this fixed, non-secret route.
    if page.url().map_err(display_error)?.as_str() == "https://chatgpt.com/?temporary-chat=true" {
        state["currentUrl"] = json!("https://chatgpt.com/?temporary-chat=true");
    }
    Ok(state)
}

fn ready(provider: &str, url: &str, snapshot: Option<&Value>) -> bool {
    let Some(s) = snapshot else { return false };
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port().is_some_and(|p| p != 443)
        || s["loginRequired"] == true
        || s["streaming"] == true
        || !s["messages"].as_array().is_some_and(|v| v.is_empty())
        || !s["draft"].as_str().is_some_and(|v| v.trim().is_empty())
    {
        return false;
    }
    let Some(observed) = s["url"].as_str().and_then(|v| Url::parse(v).ok()) else {
        return false;
    };
    if observed.origin() != url.origin() || observed.path() != url.path() {
        return false;
    }
    if provider == "chatgpt" {
        url.host_str() == Some("chatgpt.com")
            && url.path() == "/"
            && url.query() == Some("temporary-chat=true")
            && (s["composerReady"] == true || s["privateSendReady"] == true)
    } else {
        matches!(url.host_str(), Some("google.com" | "www.google.com"))
            && url.path() == "/aimode"
            && s["composerReady"] == true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_labels_preserve_owner_and_do_not_alias_personal_windows() {
        let a = label(&CHATGPT, "owner1", "00000000-0000-4000-8000-000000000001").unwrap();
        assert_ne!(a, window_label(&CHATGPT, "owner1"));
        assert_ne!(
            a,
            label(&CHATGPT, "owner2", "00000000-0000-4000-8000-000000000001").unwrap()
        );
        assert!(label(&CHATGPT, "owner1", "../main").is_err());
    }
    #[test]
    fn group_send_requires_an_empty_bound_temporary_document() {
        let mut s =
            json!({"url":"https://chatgpt.com/","messages":[],"draft":"","composerReady":true});
        assert!(ready(
            "chatgpt",
            "https://chatgpt.com/?temporary-chat=true",
            Some(&s)
        ));
        assert!(!ready("chatgpt", "https://chatgpt.com/", Some(&s)));
        s["messages"] = json!([{"role":"user","content":"personal"}]);
        assert!(!ready(
            "chatgpt",
            "https://chatgpt.com/?temporary-chat=true",
            Some(&s)
        ));
    }
}
