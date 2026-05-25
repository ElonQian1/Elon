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
    if let Some(existing) = value
        .get("apk_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty() && *url != "null")
    {
        return (raw, Some(existing.to_string()));
    }

    let Some(apk_url) = latest_project_apk_url(download_base, workspaces) else {
        return (raw, None);
    };
    let Some(object) = value.as_object_mut() else {
        return (raw, None);
    };
    object.insert("apk_url".into(), Value::String(apk_url.clone()));

    (
        serde_json::to_string(&value).unwrap_or_else(|_| raw),
        Some(apk_url),
    )
}

pub(crate) fn latest_project_apk_url(download_base: &str, workspaces: &[&Path]) -> Option<String> {
    workspaces
        .iter()
        .any(|workspace| tools::find_latest_apk(workspace).is_some())
        .then(|| tools::stable_apk_url(download_base))
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
}
