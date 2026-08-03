use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::sync::Notify;

use super::broker::LiveUiSession;

const LIST_TOOL: &str = "ui_list_design_events";
const MAX_EVENT_BYTES: u64 = 64 * 1024;
const MAX_STORED_EVENTS: usize = 1_000;
static EVENT_NOTIFY: OnceLock<Notify> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesignEvent {
    schema_version: u32,
    cursor: String,
    event_id: String,
    event_type: String,
    tool: String,
    task_id: Option<String>,
    design_session_id: Option<String>,
    draft_id: Option<String>,
    platform: Option<String>,
    route: Option<String>,
    revision: Option<u64>,
    created_at: String,
    payload: Value,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![json!({
        "name":LIST_TOOL,
        "description":"按 taskId 和单调 cursor 增量读取项目后台设计事件；可进行最长 15 秒的有界等待。",
        "inputSchema":{"type":"object","additionalProperties":false,"properties":{
            "taskId":{"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"},
            "afterCursor":{"type":"string","maxLength":96},
            "limit":{"type":"integer","minimum":1,"maximum":100},
            "waitMs":{"type":"integer","minimum":0,"maximum":15000}
        }},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
    })]
}

pub(super) fn is_tool(name: &str) -> bool {
    name == LIST_TOOL
}

pub(super) async fn call(session: &LiveUiSession, arguments: Value) -> Result<Value> {
    let root = canonical_root(session)?;
    let after_cursor = optional_text(&arguments, "afterCursor").unwrap_or("");
    validate_cursor(after_cursor)?;
    let task_id = optional_text(&arguments, "taskId");
    if let Some(task_id) = task_id {
        super::design_task_binding::validate_task_id(task_id)?;
    }
    let limit = arguments.get("limit").and_then(Value::as_u64).unwrap_or(50) as usize;
    if !(1..=100).contains(&limit) {
        bail!("limit 必须在 1..100 之间");
    }
    let wait_ms = arguments.get("waitMs").and_then(Value::as_u64).unwrap_or(0);
    if wait_ms > 15_000 {
        bail!("waitMs 必须在 0..15000 之间");
    }

    let notified = notify().notified();
    let mut page = read_page(&root, after_cursor, task_id, limit)?;
    if page.events.is_empty() && wait_ms > 0 {
        let _ = tokio::time::timeout(Duration::from_millis(wait_ms), notified).await;
        page = read_page(&root, after_cursor, task_id, limit)?;
    }
    Ok(json!({
        "schema":"elon.ui-design-events.v1",
        "events":page.events,
        "cursor":page.cursor,
        "hasMore":page.has_more,
        "waited":wait_ms > 0,
        "contentEmbedded":false
    }))
}

pub(super) fn record_tool_event(
    session: &LiveUiSession,
    tool: &str,
    arguments: &Value,
    result: &Value,
) -> Result<()> {
    let Some(event_type) = event_type(tool) else {
        return Ok(());
    };
    let root = canonical_root(session)?;
    let design_session_id = first_text(
        result,
        &[
            "/binding/designSessionId",
            "/designSessionId",
            "/session/designSessionId",
            "/designSession/designSessionId",
            "/draft/designSessionId",
        ],
    )
    .or_else(|| optional_text(arguments, "designSessionId"));
    let task_id = first_text(result, &["/binding/taskId"])
        .or_else(|| optional_text(arguments, "taskId"))
        .map(str::to_string)
        .or_else(|| {
            design_session_id.and_then(|id| {
                super::design_task_binding::find_active_for_session(session, id)
                    .map(|binding| binding.task_id)
            })
        });
    let now = chrono::Utc::now();
    let id = uuid::Uuid::new_v4().simple().to_string();
    let cursor = format!("{:020}_{id}", now.timestamp_millis());
    let event = DesignEvent {
        schema_version: 1,
        cursor: cursor.clone(),
        event_id: format!("event_{id}"),
        event_type: event_type.to_string(),
        tool: tool.to_string(),
        task_id,
        design_session_id: design_session_id.map(str::to_string),
        draft_id: first_text(result, &["/draft/draftId", "/draftId"])
            .or_else(|| optional_text(arguments, "draftId"))
            .map(str::to_string),
        platform: first_text(
            result,
            &["/session/platform", "/designSession/platform", "/platform"],
        )
        .map(str::to_string),
        route: first_text(
            result,
            &["/session/route", "/designSession/route", "/route"],
        )
        .map(str::to_string),
        revision: result
            .pointer("/draft/revision")
            .and_then(Value::as_u64)
            .or_else(|| result.get("revision").and_then(Value::as_u64)),
        created_at: now.to_rfc3339(),
        payload: compact_payload(result),
    };
    let directory = event_directory(&root, true)?;
    fs::write(
        directory.join(format!("{cursor}.json")),
        serde_json::to_vec(&event)?,
    )?;
    prune(&directory)?;
    notify().notify_waiters();
    Ok(())
}

struct EventPage {
    events: Vec<DesignEvent>,
    cursor: String,
    has_more: bool,
}

fn read_page(root: &Path, after: &str, task_id: Option<&str>, limit: usize) -> Result<EventPage> {
    let directory = event_directory(root, false)?;
    if !directory.is_dir() {
        return Ok(EventPage {
            events: Vec::new(),
            cursor: after.to_string(),
            has_more: false,
        });
    }
    let mut paths = fs::read_dir(directory)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|v| v.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut events = Vec::new();
    let mut has_more = false;
    for path in paths {
        let cursor = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if cursor <= after {
            continue;
        }
        let event = fs::metadata(&path)
            .ok()
            .filter(|metadata| metadata.is_file() && metadata.len() <= MAX_EVENT_BYTES)
            .and_then(|_| fs::read(&path).ok())
            .and_then(|bytes| serde_json::from_slice::<DesignEvent>(&bytes).ok());
        let Some(event) = event else {
            continue;
        };
        if task_id.is_some_and(|task_id| event.task_id.as_deref() != Some(task_id)) {
            continue;
        }
        if events.len() == limit {
            has_more = true;
            break;
        }
        events.push(event);
    }
    let cursor = events
        .last()
        .map(|event| event.cursor.clone())
        .unwrap_or_else(|| after.to_string());
    Ok(EventPage {
        events,
        cursor,
        has_more,
    })
}

fn compact_payload(result: &Value) -> Value {
    let mut payload = Map::new();
    for (name, pointers) in [
        ("action", &["/action"] as &[&str]),
        ("status", &["/status", "/draft/status", "/receipt/status"]),
        ("sourceModified", &["/sourceModified"]),
        ("candidateCount", &["/candidateCount"]),
        ("overallStatus", &["/overallStatus"]),
        (
            "artifactSha256",
            &["/artifact/sha256", "/capture/artifact/sha256"],
        ),
        (
            "uiTreeSha256",
            &["/uiTree/sha256", "/capture/uiTree/sha256"],
        ),
    ] {
        if let Some(value) = pointers
            .iter()
            .find_map(|pointer| result.pointer(pointer))
            .cloned()
        {
            payload.insert(name.to_string(), value);
        }
    }
    Value::Object(payload)
}

fn event_type(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "ui_bind_design_task" => "TASK_BOUND",
        "ui_renew_design_task_binding" => "TASK_LEASE_RENEWED",
        "ui_settle_design_task_binding" => "TASK_SETTLED",
        "ui_open_design_target" => "SESSION_OPENED",
        "ui_capture_design_surface" => "SURFACE_CAPTURED",
        "ui_prepare_design_browser" | "ui_interact_design_browser" | "ui_stop_design_browser" => {
            "BROWSER_UPDATED"
        }
        "ui_prepare_tauri_runtime"
        | "ui_capture_tauri_host"
        | "ui_stop_tauri_runtime"
        | "ui_capture_tauri_behavior" => "TAURI_UPDATED",
        "ui_create_design_draft" | "ui_update_design_draft" | "ui_undo_design_draft" => {
            "DRAFT_UPDATED"
        }
        "ui_preview_design_draft" => "DRAFT_PREVIEWED",
        "ui_restore_design_draft_preview" => "DRAFT_PREVIEW_RESTORED",
        "ui_suggest_design_source_binding" => "SOURCE_BINDING_SUGGESTED",
        "ui_begin_design_writeback" => "WRITEBACK_STARTED",
        "ui_complete_design_writeback" => "WRITEBACK_UPDATED",
        _ => return None,
    })
}

fn prune(directory: &Path) -> Result<()> {
    let mut paths = fs::read_dir(directory)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let remove_count = paths.len().saturating_sub(MAX_STORED_EVENTS);
    for path in paths.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

fn event_directory(root: &Path, create: bool) -> Result<PathBuf> {
    let directory = root.join(".elon/ui-tuner/headless-design/events");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.exists() {
        return Ok(directory);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("后台设计事件目录越出项目");
    }
    Ok(canonical)
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("后台设计事件需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}

fn notify() -> &'static Notify {
    EVENT_NOTIFY.get_or_init(Notify::new)
}

fn optional_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn first_text<'a>(value: &'a Value, pointers: &[&str]) -> Option<&'a str> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn validate_cursor(value: &str) -> Result<()> {
    if value.len() > 96
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        bail!("afterCursor 无效");
    }
    Ok(())
}
