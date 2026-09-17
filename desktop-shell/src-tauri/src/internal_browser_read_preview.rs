//! Only a fixed, read-only adapter runs in the isolated original-page WebView.
use serde_json::Value;
use std::{sync::mpsc, time::Duration};
use tauri::Webview;

const ADAPTER: &str =
    include_str!("../../../android/app/src/main/assets/social_link_read_adapter.js");

pub(crate) async fn read(tab: &Webview, original: &str) -> Option<Value> {
    let source =
        crate::external_navigation::validate_external_url(&original.parse::<tauri::Url>().ok()?);
    if original.len() > 4096 || source.is_err() {
        return None;
    }
    let script = format!(
        "{}.read({})",
        ADAPTER,
        serde_json::to_string(original).ok()?
    );
    let (tx, rx) = mpsc::sync_channel(1);
    tab.eval_with_callback(script, move |value| {
        let _ = tx.try_send(value);
    })
    .ok()?;
    let encoded =
        tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(2)))
            .await
            .ok()?
            .ok()?;
    if encoded.len() > 16384 {
        return None;
    }
    let result: Value = serde_json::from_str(&encoded).ok()?;
    if result.get("schema")?.as_u64()? != 1
        || result.get("original")?.as_str()? != original
        || !result.get("article")?.as_bool()?
    {
        return None;
    }
    // Forward only the adapter's public metadata fields, never arbitrary page data.
    Some(serde_json::json!({
        "schema": 1, "original": original, "article": true,
        "url": result.get("url")?.as_str()?, "title": result.get("title")?.as_str()?,
        "author": result.get("author")?.as_str()?,
        "description": result.get("description").and_then(|v| v.as_str()).unwrap_or(""),
        "image": result.get("image")?
    }))
}
