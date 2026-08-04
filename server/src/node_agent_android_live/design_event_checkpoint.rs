use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::broker::LiveUiSession;

const GET_TOOL: &str = "ui_get_design_event_checkpoint";
const COMMIT_TOOL: &str = "ui_commit_design_event_checkpoint";
const MAX_RECORD_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EventCheckpoint {
    schema_version: u32,
    consumer_id: String,
    task_id: String,
    cursor: String,
    revision: u64,
    created_at: String,
    updated_at: String,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            GET_TOOL,
            "读取指定 consumerId/taskId 的持久设计事件 checkpoint，使 PC 或代理重启后从已确认 cursor 继续。",
            checkpoint_schema(false),
            true,
        ),
        tool(
            COMMIT_TOOL,
            "以 expectedRevision 单调提交 consumerId/taskId 的设计事件 cursor；只确认已处理事件，不删除事件。",
            checkpoint_schema(true),
            false,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, GET_TOOL | COMMIT_TOOL)
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    let root = canonical_root(session)?;
    let consumer_id = required_text(&arguments, "consumerId")?;
    validate_consumer_id(consumer_id)?;
    let task_id = required_text(&arguments, "taskId")?;
    super::design_task_binding::validate_task_id(task_id)?;
    match name {
        GET_TOOL => get(&root, consumer_id, task_id),
        COMMIT_TOOL => commit(session, &root, consumer_id, task_id, &arguments),
        _ => bail!("未知设计事件 checkpoint 工具: {name}"),
    }
}

fn get(root: &Path, consumer_id: &str, task_id: &str) -> Result<Value> {
    let checkpoint = read(root, consumer_id, task_id)?;
    Ok(json!({
        "schema":"elon.ui-design-event-checkpoint.v1",
        "checkpoint":checkpoint,
        "resumeAfterCursor":checkpoint.as_ref().map(|value| value.cursor.as_str()).unwrap_or(""),
        "revision":checkpoint.as_ref().map(|value| value.revision).unwrap_or(0),
        "contentEmbedded":false
    }))
}

fn commit(
    session: &LiveUiSession,
    root: &Path,
    consumer_id: &str,
    task_id: &str,
    arguments: &Value,
) -> Result<Value> {
    let cursor = required_text(arguments, "cursor")?;
    super::design_event_stream::validate_cursor(cursor)?;
    if !super::design_event_stream::cursor_belongs_to_task(session, cursor, task_id)? {
        bail!("DESIGN_EVENT_CURSOR_MISMATCH：cursor 不存在或不属于指定 taskId");
    }
    let expected = arguments
        .get("expectedRevision")
        .and_then(Value::as_u64)
        .context("缺少 expectedRevision")?;
    let current = read(root, consumer_id, task_id)?;
    let current_revision = current.as_ref().map(|value| value.revision).unwrap_or(0);
    if current_revision != expected {
        bail!("DESIGN_EVENT_CHECKPOINT_CONFLICT：expected={expected} actual={current_revision}");
    }
    if current
        .as_ref()
        .is_some_and(|value| value.cursor.as_str() > cursor)
    {
        bail!("DESIGN_EVENT_CHECKPOINT_REGRESSION：cursor 不能倒退");
    }
    if let Some(current) = current.as_ref().filter(|value| value.cursor == cursor) {
        return Ok(
            json!({"schema":"elon.ui-design-event-checkpoint.v1","action":"UNCHANGED","checkpoint":current}),
        );
    }
    let now = chrono::Utc::now().to_rfc3339();
    let checkpoint = EventCheckpoint {
        schema_version: 1,
        consumer_id: consumer_id.to_string(),
        task_id: task_id.to_string(),
        cursor: cursor.to_string(),
        revision: current_revision + 1,
        created_at: current
            .as_ref()
            .map(|value| value.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    };
    persist(root, &checkpoint)?;
    Ok(
        json!({"schema":"elon.ui-design-event-checkpoint.v1","action":"COMMITTED","checkpoint":checkpoint}),
    )
}

fn read(root: &Path, consumer_id: &str, task_id: &str) -> Result<Option<EventCheckpoint>> {
    let path = checkpoint_path(root, consumer_id, task_id, false)?;
    if !path.is_file() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path)?;
    if metadata.len() > MAX_RECORD_BYTES {
        bail!("设计事件 checkpoint 超过大小上限");
    }
    Ok(Some(
        serde_json::from_slice(&fs::read(path)?).context("设计事件 checkpoint JSON 无效")?,
    ))
}

fn persist(root: &Path, checkpoint: &EventCheckpoint) -> Result<()> {
    let path = checkpoint_path(root, &checkpoint.consumer_id, &checkpoint.task_id, true)?;
    fs::write(path, serde_json::to_vec_pretty(checkpoint)?)?;
    Ok(())
}

fn checkpoint_path(root: &Path, consumer_id: &str, task_id: &str, create: bool) -> Result<PathBuf> {
    let directory = checkpoint_directory(root, create)?;
    let key = format!("{consumer_id}\0{task_id}");
    let digest = hex::encode(Sha256::digest(key.as_bytes()));
    Ok(directory.join(format!("{digest}.json")))
}

fn checkpoint_directory(root: &Path, create: bool) -> Result<PathBuf> {
    let directory = root.join(".elon/ui-tuner/headless-design/event-checkpoints");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.exists() {
        return Ok(directory);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("设计事件 checkpoint 目录越出项目");
    }
    Ok(canonical)
}

fn validate_consumer_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "._:-".contains(ch))
    {
        bail!("consumerId 必须为 1..100 位 ASCII 字母、数字或 ._:-");
    }
    Ok(())
}

fn checkpoint_schema(commit: bool) -> Value {
    let mut properties = json!({
        "consumerId":{"type":"string","minLength":1,"maxLength":100,"pattern":"^[A-Za-z0-9._:-]+$"},
        "taskId":{"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"}
    });
    let mut required = vec!["consumerId", "taskId"];
    if commit {
        properties["cursor"] = json!({"type":"string","minLength":1,"maxLength":96});
        properties["expectedRevision"] = json!({"type":"integer","minimum":0});
        required.extend(["cursor", "expectedRevision"]);
    }
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("设计事件 checkpoint 需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{
        "readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}
