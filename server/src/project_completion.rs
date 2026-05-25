use std::path::Path;

use serde_json::Value;

use crate::tools;

pub(crate) fn ensure_done_event_has_project_apk_url(
    raw: String,
    download_base: &str,
    workspaces: &[&Path],
) -> (String, Option<String>) {
    let Ok(mut value) = serde_json::from_str::<Value>(&raw) else {
        return (raw, None);
    };
    if value.get("type").and_then(Value::as_str) != Some("done") {
        return (raw, None);
    }
    let mut changed = sanitize_done_message(&mut value);
    let existing_apk_url = value
        .get("apk_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty() && *url != "null")
        .map(ToOwned::to_owned);
    if let Some(existing) = existing_apk_url {
        return (serialize_done_event(value, raw, changed), Some(existing));
    }

    let Some(apk_url) = latest_project_apk_url(download_base, workspaces) else {
        return (serialize_done_event(value, raw, changed), None);
    };
    let Some(object) = value.as_object_mut() else {
        return (serialize_done_event(value, raw, changed), None);
    };
    object.insert("apk_url".into(), Value::String(apk_url.clone()));
    changed = true;

    (serialize_done_event(value, raw, changed), Some(apk_url))
}

pub(crate) fn latest_project_apk_url(download_base: &str, workspaces: &[&Path]) -> Option<String> {
    workspaces
        .iter()
        .any(|workspace| tools::find_latest_apk(workspace).is_some())
        .then(|| tools::stable_apk_url(download_base))
}

fn serialize_done_event(value: Value, original: String, changed: bool) -> String {
    if changed {
        serde_json::to_string(&value).unwrap_or(original)
    } else {
        original
    }
}

fn sanitize_done_message(value: &mut Value) -> bool {
    let Some(message) = value.get("message").and_then(Value::as_str) else {
        return false;
    };
    let sanitized = sanitize_done_message_text(message);
    if sanitized == message {
        return false;
    }
    let Some(object) = value.as_object_mut() else {
        return false;
    };
    object.insert("message".into(), Value::String(sanitized));
    true
}

fn sanitize_done_message_text(message: &str) -> String {
    let mut lines = Vec::new();
    for line in message.lines() {
        let sanitized = sanitize_no_remote_push_fragments(line);
        if is_no_remote_push_notice(sanitized.trim()) {
            continue;
        }
        if sanitized.trim().is_empty() && !line.trim().is_empty() {
            continue;
        }
        lines.push(sanitized.trim_end().to_string());
    }

    let mut cleaned = lines.join("\n").trim().to_string();
    while cleaned.contains("\n\n\n") {
        cleaned = cleaned.replace("\n\n\n", "\n\n");
    }
    cleaned
}

fn sanitize_no_remote_push_fragments(line: &str) -> String {
    let mut cleaned = line.to_string();
    for fragment in [
        "当前仓库没有配置远端，所以无法 push。",
        "当前仓库没有配置远端，无法 push。",
        "当前仓库未配置远端，所以无法 push。",
        "当前仓库未配置远端，无法 push。",
        "仓库没有配置远端，所以无法 push。",
        "没有配置远端，所以无法 push。",
        "当前仓库没有 origin 远端，所以无法 push。",
        "当前仓库未配置 origin，所以无法 push。",
        "当前仓库没有配置远端，所以无法推送。",
        "当前仓库未配置远端，所以无法推送。",
        "（无远程或 push 失败，仅本地提交）",
        "(无远程或 push 失败，仅本地提交)",
        " (无远程或 push 失败，仅本地提交)",
    ] {
        cleaned = cleaned.replace(fragment, "");
    }
    cleaned
}

fn is_no_remote_push_notice(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let mentions_push = lower.contains("push") || line.contains("推送");
    let mentions_no_remote = line.contains("没有配置远端")
        || line.contains("未配置远端")
        || line.contains("无远程")
        || line.contains("无远端")
        || line.contains("没有 origin")
        || line.contains("未配置 origin")
        || lower.contains("no remote")
        || lower.contains("remote not configured")
        || lower.contains("origin not configured");
    mentions_push && mentions_no_remote
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_existing_done_apk_url() {
        let raw = r#"{"type":"done","message":"ok","apk_url":"https://example.test/app.apk"}"#;
        let (updated, apk_url) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);

        assert_eq!(updated, raw);
        assert_eq!(apk_url.as_deref(), Some("https://example.test/app.apk"));
    }

    #[test]
    fn leaves_non_done_events_unchanged() {
        let raw = r#"{"type":"progress","message":"working"}"#;
        let (updated, apk_url) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);

        assert_eq!(updated, raw);
        assert!(apk_url.is_none());
    }

    #[test]
    fn fills_missing_done_apk_url_when_artifact_exists() {
        let root = std::env::temp_dir().join(format!(
            "elon_project_completion_test_{}",
            std::process::id()
        ));
        let apk_dir = root.join("android/app/build/outputs/apk/release");
        std::fs::create_dir_all(&apk_dir).unwrap();
        std::fs::write(apk_dir.join("app-release.apk"), b"apk").unwrap();

        let raw = r#"{"type":"done","message":"ok","apk_url":null}"#;
        let (updated, apk_url) = ensure_done_event_has_project_apk_url(
            raw.into(),
            "https://download.test/project",
            &[&root],
        );
        let value: Value = serde_json::from_str(&updated).unwrap();

        assert_eq!(
            apk_url.as_deref(),
            Some("https://download.test/project/latest.apk")
        );
        assert_eq!(
            value.get("apk_url").and_then(Value::as_str),
            Some("https://download.test/project/latest.apk")
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn removes_no_remote_push_notice_from_done_message() {
        let raw = r#"{"type":"done","message":"已完成并提交：9179ecf Build simple role selection app。\n\n当前仓库没有配置远端，所以无法 push。\n\n下载新 APK：https://example.test/latest.apk","apk_url":"https://example.test/latest.apk"}"#;
        let (updated, apk_url) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);
        let value: Value = serde_json::from_str(&updated).unwrap();
        let message = value.get("message").and_then(Value::as_str).unwrap();

        assert_eq!(apk_url.as_deref(), Some("https://example.test/latest.apk"));
        assert!(message.contains("已完成并提交"));
        assert!(message.contains("下载新 APK"));
        assert!(!message.contains("无法 push"));
        assert!(!message.contains("没有配置远端"));
    }

    #[test]
    fn keeps_commit_summary_when_removing_parenthetical_remote_noise() {
        let raw = r#"{"type":"done","message":"已完成并提交：9179ecf（无远程或 push 失败，仅本地提交）","apk_url":"https://example.test/latest.apk"}"#;
        let (updated, _) =
            ensure_done_event_has_project_apk_url(raw.into(), "https://download.test", &[]);
        let value: Value = serde_json::from_str(&updated).unwrap();
        let message = value.get("message").and_then(Value::as_str).unwrap();

        assert_eq!(message, "已完成并提交：9179ecf");
    }
}
