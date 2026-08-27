use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Webview};

#[path = "research_capture/analysis.rs"]
mod analysis;
#[path = "research_capture/private_observation.rs"]
mod private_observation;

use super::{
    ensure_main_webview, profile_directory, provider, provider_for_window_label,
    resolve_owner_fingerprint, ProviderDefinition, LOCAL_AI_WINDOW_PREFIX,
};

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_CAPTURE_COUNT: usize = 256;
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
pub(super) const DIRECTORY_NAME: &str = "research-captures";

#[tauri::command]
pub(crate) async fn publish_local_ai_web_research_capture(
    app: AppHandle,
    webview: Webview,
    capture: ResearchCapture,
) -> Result<ResearchCaptureReceipt, String> {
    let label = webview.label().to_string();
    let provider = provider_for_window_label(&label)
        .filter(|provider| provider.adapter.is_some())
        .ok_or_else(|| "原始响应研究采样只允许已登记的本地 AI 会话窗口发送。".to_string())?;
    let root = root_for_label(&app, provider, &label)?;
    let provider_id = provider.id;
    tauri::async_runtime::spawn_blocking(move || store(&root, provider_id, capture))
        .await
        .map_err(|error| format!("本机研究响应保存任务异常结束：{error}"))?
}

#[tauri::command]
pub(crate) fn open_local_ai_web_research_directory(
    app: AppHandle,
    webview: Webview,
    provider_id: String,
    owner_key: String,
) -> Result<(), String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    let root = profile_directory(&app, provider, &fingerprint)?.join(DIRECTORY_NAME);
    fs::create_dir_all(&root).map_err(display_error)?;
    open_directory(&root)
}

#[tauri::command]
pub(crate) fn get_local_ai_web_research_capture_status(
    app: AppHandle,
    webview: Webview,
    provider_id: String,
    owner_key: String,
) -> Result<analysis::ResearchCaptureStatus, String> {
    ensure_main_webview(&webview)?;
    let provider = provider(&provider_id)?;
    let fingerprint = resolve_owner_fingerprint(&app, provider, &owner_key)?;
    let root = profile_directory(&app, provider, &fingerprint)?.join(DIRECTORY_NAME);
    analysis::read_status(&root, provider.id)
}

pub(super) fn record_sanitized_adapter_observation(
    app: &AppHandle,
    provider: &ProviderDefinition,
    label: &str,
    kind: &str,
    payload: &serde_json::Value,
) -> bool {
    let Some(observation) = private_observation::parse(provider.id, kind, payload) else {
        return false;
    };
    if let Ok(root) = root_for_label(app, provider, label) {
        let _ = private_observation::store(&root, provider.id, observation, now_ms());
    }
    true
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchCapture {
    pub provider_id: String,
    pub method: String,
    pub endpoint_family: String,
    pub transport: String,
    pub status: u16,
    pub format: String,
    pub captured_at_ms: u64,
    pub body: String,
    pub truncated: bool,
    pub analysis: Option<analysis::CaptureAnalysis>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchCaptureReceipt {
    capture_id: String,
    stored: bool,
    deduplicated: bool,
    truncated: bool,
    size_bytes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureMetadata<'a> {
    schema: &'static str,
    capture_id: &'a str,
    provider_id: &'a str,
    method: &'a str,
    endpoint_family: &'a str,
    transport: &'a str,
    status: u16,
    format: &'a str,
    captured_at_ms: u64,
    stored_at_ms: u64,
    body_sha256: &'a str,
    size_bytes: usize,
    truncated: bool,
    analysis: Option<&'a analysis::CaptureAnalysis>,
}

struct StoredBody {
    body: String,
    truncated: bool,
}

pub(super) fn store(
    root: &Path,
    expected_provider_id: &str,
    capture: ResearchCapture,
) -> Result<ResearchCaptureReceipt, String> {
    validate(expected_provider_id, &capture)?;
    let stored = bounded_body(capture.body, capture.truncated);
    if stored.body.is_empty() {
        return Err("研究响应正文为空，未保存。".to_string());
    }
    fs::create_dir_all(root).map_err(display_error)?;
    cleanup(root)?;

    let body_sha256 = sha256_hex(stored.body.as_bytes());
    let capture_id = sha256_hex(format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}",
        expected_provider_id,
        capture.method,
        capture.endpoint_family,
        capture.transport,
        capture.status,
        capture.format,
        body_sha256,
    ).as_bytes());
    let extension = extension_for(&capture.format);
    let body_path = root.join(format!("{capture_id}.{extension}"));
    let metadata_path = root.join(format!("{capture_id}.meta.json"));
    let deduplicated = body_path.is_file();
    if !deduplicated {
        write_new(&body_path, stored.body.as_bytes())?;
    }

    let metadata = CaptureMetadata {
        schema: "yilong.web-ai.research-capture.v1",
        capture_id: &capture_id,
        provider_id: expected_provider_id,
        method: &capture.method,
        endpoint_family: &capture.endpoint_family,
        transport: &capture.transport,
        status: capture.status,
        format: &capture.format,
        captured_at_ms: capture.captured_at_ms,
        stored_at_ms: now_ms(),
        body_sha256: &body_sha256,
        size_bytes: stored.body.len(),
        truncated: stored.truncated,
        analysis: capture.analysis.as_ref(),
    };
    if !metadata_path.is_file() {
        let encoded = serde_json::to_vec_pretty(&metadata).map_err(display_error)?;
        write_new(&metadata_path, &encoded)?;
    }
    cleanup(root)?;
    Ok(ResearchCaptureReceipt {
        capture_id,
        stored: true,
        deduplicated,
        truncated: stored.truncated,
        size_bytes: stored.body.len(),
    })
}

pub(super) fn clear(root: &Path) -> Result<(), String> {
    if root.is_dir() {
        fs::remove_dir_all(root).map_err(display_error)?;
    }
    Ok(())
}

fn validate(expected_provider_id: &str, capture: &ResearchCapture) -> Result<(), String> {
    if capture.provider_id != expected_provider_id {
        return Err("研究响应厂商与当前 WebView 不匹配。".to_string());
    }
    if !matches!(capture.method.as_str(), "GET" | "POST")
        || !matches!(capture.transport.as_str(), "fetch" | "xhr")
        || !(100..=599).contains(&capture.status)
        || !matches!(capture.format.as_str(), "sse" | "json" | "ndjson" | "text")
    {
        return Err("研究响应元数据无效。".to_string());
    }
    let family_allowed = match expected_provider_id {
        "chatgpt" => matches!(
            capture.endpoint_family.as_str(),
            "conversation_stream" | "conversation_detail"
        ),
        "google-ai-mode" => capture.endpoint_family == "ai_rpc",
        _ => false,
    };
    if !family_allowed {
        return Err("研究响应接口族不在当前开发范围内。".to_string());
    }
    analysis::validate(capture.analysis.as_ref())?;
    Ok(())
}

fn root_for_label(
    app: &AppHandle,
    provider: &ProviderDefinition,
    label: &str,
) -> Result<PathBuf, String> {
    let prefix = format!("{LOCAL_AI_WINDOW_PREFIX}{}-", provider.id);
    let fingerprint = label
        .strip_prefix(&prefix)
        .filter(|value| matches!(value.len(), 16 | 32))
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| "本地 AI 研究采样窗口身份无效。".to_string())?;
    profile_directory(app, provider, fingerprint).map(|profile| profile.join(DIRECTORY_NAME))
}

fn bounded_body(body: String, already_truncated: bool) -> StoredBody {
    if body.len() <= MAX_BODY_BYTES {
        return StoredBody { body, truncated: already_truncated };
    }
    let mut boundary = MAX_BODY_BYTES;
    while !body.is_char_boundary(boundary) {
        boundary -= 1;
    }
    StoredBody { body: body[..boundary].to_string(), truncated: true }
}

fn cleanup(root: &Path) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }
    let now = SystemTime::now();
    let mut entries = fs::read_dir(root)
        .map_err(display_error)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !is_body_path(&path) { return None; }
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
            Some((path, modified, metadata.len()))
        })
        .collect::<Vec<_>>();

    for (path, modified, _) in &entries {
        if now.duration_since(*modified).unwrap_or_default() > MAX_AGE {
            remove_capture(path);
        }
    }
    entries.retain(|(path, _, _)| path.exists());
    entries.sort_by_key(|(_, modified, _)| *modified);
    let mut total = entries.iter().map(|(_, _, size)| *size).sum::<u64>();
    while entries.len() > MAX_CAPTURE_COUNT || total > MAX_TOTAL_BYTES {
        let (path, _, size) = entries.remove(0);
        total = total.saturating_sub(size);
        remove_capture(&path);
    }
    Ok(())
}

fn is_body_path(path: &Path) -> bool {
    path.is_file()
        && matches!(path.extension().and_then(|value| value.to_str()), Some("sse" | "json" | "ndjson" | "txt"))
        && !path.file_name().and_then(|value| value.to_str()).is_some_and(|name| name.ends_with(".meta.json"))
}

fn remove_capture(body_path: &Path) {
    let Some(stem) = body_path.file_stem().and_then(|value| value.to_str()) else { return; };
    let metadata_path: PathBuf = body_path.with_file_name(format!("{stem}.meta.json"));
    let _ = fs::remove_file(body_path);
    let _ = fs::remove_file(metadata_path);
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(display_error)?;
    file.write_all(bytes).map_err(display_error)?;
    file.sync_all().map_err(display_error)
}

fn extension_for(format: &str) -> &'static str {
    match format {
        "sse" => "sse",
        "json" => "json",
        "ndjson" => "ndjson",
        _ => "txt",
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(windows)]
fn open_directory(path: &Path) -> Result<(), String> {
    use std::{os::windows::process::CommandExt, process::Command};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new("explorer.exe")
        .arg(path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开本机研究采样目录：{error}"))
}

#[cfg(not(windows))]
fn open_directory(_path: &Path) -> Result<(), String> {
    Err("研究采样目录仅在 Windows 客户端开放。".to_string())
}

fn display_error(error: impl std::fmt::Display) -> String {
    format!("无法保存本机网页 AI 研究响应：{error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_body_is_retained_and_metadata_does_not_repeat_it() {
        let root = temporary_root("raw");
        let receipt = store(&root, "chatgpt", sample("secret response body")).unwrap();
        let body = fs::read_to_string(root.join(format!("{}.sse", receipt.capture_id))).unwrap();
        let metadata = fs::read_to_string(root.join(format!("{}.meta.json", receipt.capture_id))).unwrap();
        assert_eq!(body, "secret response body");
        assert!(!metadata.contains("secret response body"));
        clear(&root).unwrap();
    }

    #[test]
    fn duplicate_body_is_deduplicated() {
        let root = temporary_root("dedupe");
        assert!(!store(&root, "chatgpt", sample("same")).unwrap().deduplicated);
        assert!(store(&root, "chatgpt", sample("same")).unwrap().deduplicated);
        clear(&root).unwrap();
    }

    #[test]
    fn stored_rich_analysis_is_visible_to_the_win_status_command() {
        let root = temporary_root("analysis");
        let mut capture = sample("data: rich response");
        capture.analysis = Some(analysis::CaptureAnalysis {
            schema: "yilong.web-ai.capture-analysis.v1".to_string(),
            analyzer_version: 2,
            policy_available: true,
            decoded_frame_count: 3,
            accepted_frame_count: 2,
            assistant_frame_count: 1,
            progress_frame_count: 0,
            text_length: 128,
            rich_kinds: vec!["finance".to_string()],
            content_types: vec!["text".to_string()],
            unsupported_rich_count: 0,
            completed: true,
            parse_error: false,
        });
        store(&root, "chatgpt", capture).unwrap();

        let status = serde_json::to_value(analysis::read_status(&root, "chatgpt").unwrap()).unwrap();
        assert_eq!(status["compatibility"], "rich_compatible");
        assert_eq!(status["acceptedFrameCount"], 2);
        assert_eq!(status["richKinds"][0], "finance");
        clear(&root).unwrap();
    }

    fn sample(body: &str) -> ResearchCapture {
        ResearchCapture {
            provider_id: "chatgpt".to_string(),
            method: "POST".to_string(),
            endpoint_family: "conversation_stream".to_string(),
            transport: "fetch".to_string(),
            status: 200,
            format: "sse".to_string(),
            captured_at_ms: 1,
            body: body.to_string(),
            truncated: false,
            analysis: None,
        }
    }

    fn temporary_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("elon-research-capture-{label}-{}", now_ms()))
    }
}
