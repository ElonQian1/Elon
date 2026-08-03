use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{broker::LiveUiSession, design_session_store};

const BIND_TOOL: &str = "ui_bind_design_task";
const GET_TOOL: &str = "ui_get_design_task_binding";
const RENEW_TOOL: &str = "ui_renew_design_task_binding";
const SETTLE_TOOL: &str = "ui_settle_design_task_binding";
const DEFAULT_LEASE_SECONDS: i64 = 900;
const MAX_RECORD_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesignTaskBinding {
    schema_version: u32,
    pub(super) task_id: String,
    pub(super) design_session_id: String,
    pub(super) draft_id: Option<String>,
    pub(super) lease_id: String,
    status: String,
    succeeded: Option<bool>,
    acquired_at: String,
    expires_at: String,
    updated_at: String,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            BIND_TOOL,
            "把 AI taskId 与项目内设计会话/草稿绑定，并获取有界独占 lease。",
            bind_schema(),
            false,
        ),
        tool(
            GET_TOOL,
            "读取项目内指定 AI taskId 的设计会话绑定；不会创建或续租。",
            task_schema(),
            true,
        ),
        tool(
            RENEW_TOOL,
            "用 leaseId 续租 AI 任务的设计会话绑定。",
            lease_schema(false),
            false,
        ),
        tool(
            SETTLE_TOOL,
            "结算 AI 任务的设计会话绑定并释放独占 lease。",
            lease_schema(true),
            false,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, BIND_TOOL | GET_TOOL | RENEW_TOOL | SETTLE_TOOL)
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    let root = canonical_root(session)?;
    let task_id = required_task_id(&arguments)?;
    match name {
        BIND_TOOL => bind(session, &root, task_id, &arguments),
        GET_TOOL => {
            Ok(json!({"schema":"elon.ui-design-task-binding.v1","binding":read(&root, task_id)?}))
        }
        RENEW_TOOL => renew(&root, task_id, &arguments),
        SETTLE_TOOL => settle(&root, task_id, &arguments),
        _ => bail!("未知设计任务绑定工具: {name}"),
    }
}

pub(super) fn find_active_for_session(
    session: &LiveUiSession,
    design_session_id: &str,
) -> Option<DesignTaskBinding> {
    let root = canonical_root(session).ok()?;
    list(&root)
        .ok()?
        .into_iter()
        .find(|binding| binding.design_session_id == design_session_id && binding.is_active())
}

fn bind(session: &LiveUiSession, root: &Path, task_id: &str, arguments: &Value) -> Result<Value> {
    let design_session_id = required_text(arguments, "designSessionId")?;
    design_session_store::validate_design_session_id(design_session_id)?;
    design_session_store::read_record(root, design_session_id).context("绑定的设计会话不存在")?;
    let draft_id = optional_text(arguments, "draftId");
    if let Some(draft_id) = draft_id {
        let result = super::design_drafts::call(
            session,
            "ui_get_design_draft",
            json!({"draftId":draft_id}),
        )?;
        if result
            .pointer("/draft/designSessionId")
            .and_then(Value::as_str)
            != Some(design_session_id)
        {
            bail!("draftId 不属于指定 designSessionId");
        }
    }
    let expected = optional_text(arguments, "expectedLeaseId");
    if let Some(current) = read(root, task_id)? {
        if current.is_active() {
            if current.design_session_id == design_session_id && expected.is_none() {
                return Ok(binding_result("UNCHANGED", current));
            }
            if expected != Some(current.lease_id.as_str()) {
                bail!("TASK_DESIGN_LEASE_CONFLICT：重新绑定需要当前 expectedLeaseId");
            }
        }
    }
    if let Some(owner) = list(root)?.into_iter().find(|binding| {
        binding.task_id != task_id
            && binding.design_session_id == design_session_id
            && binding.is_active()
    }) {
        bail!(
            "DESIGN_SESSION_LEASED：设计会话正由任务 {} 使用",
            owner.task_id
        );
    }
    let lease_seconds = lease_seconds(arguments)?;
    let now = Utc::now();
    let binding = DesignTaskBinding {
        schema_version: 1,
        task_id: task_id.to_string(),
        design_session_id: design_session_id.to_string(),
        draft_id: draft_id.map(str::to_string),
        lease_id: format!("lease_{}", uuid::Uuid::new_v4().simple()),
        status: "ACTIVE".to_string(),
        succeeded: None,
        acquired_at: now.to_rfc3339(),
        expires_at: (now + Duration::seconds(lease_seconds)).to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };
    persist(root, &binding)?;
    Ok(binding_result("BOUND", binding))
}

fn renew(root: &Path, task_id: &str, arguments: &Value) -> Result<Value> {
    let mut binding = require_lease(root, task_id, arguments)?;
    if !binding.is_active() {
        bail!("TASK_DESIGN_LEASE_EXPIRED：绑定已结算或过期");
    }
    let now = Utc::now();
    binding.expires_at = (now + Duration::seconds(lease_seconds(arguments)?)).to_rfc3339();
    binding.updated_at = now.to_rfc3339();
    persist(root, &binding)?;
    Ok(binding_result("RENEWED", binding))
}

fn settle(root: &Path, task_id: &str, arguments: &Value) -> Result<Value> {
    let mut binding = require_lease(root, task_id, arguments)?;
    if binding.status == "SETTLED" {
        return Ok(binding_result("UNCHANGED", binding));
    }
    binding.status = "SETTLED".to_string();
    binding.succeeded = arguments.get("succeeded").and_then(Value::as_bool);
    binding.updated_at = Utc::now().to_rfc3339();
    persist(root, &binding)?;
    Ok(binding_result("SETTLED", binding))
}

fn require_lease(root: &Path, task_id: &str, arguments: &Value) -> Result<DesignTaskBinding> {
    let lease_id = required_text(arguments, "leaseId")?;
    let binding = read(root, task_id)?.context("设计任务尚未绑定")?;
    if binding.lease_id != lease_id {
        bail!("TASK_DESIGN_LEASE_MISMATCH：leaseId 不匹配");
    }
    Ok(binding)
}

fn binding_result(action: &str, binding: DesignTaskBinding) -> Value {
    json!({"schema":"elon.ui-design-task-binding.v1","action":action,"binding":binding})
}

fn read(root: &Path, task_id: &str) -> Result<Option<DesignTaskBinding>> {
    let path = record_path(root, task_id, false)?;
    if !path.is_file() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path)?;
    if metadata.len() > MAX_RECORD_BYTES {
        bail!("设计任务绑定记录超过大小上限");
    }
    Ok(Some(
        serde_json::from_slice(&fs::read(path)?).context("设计任务绑定 JSON 无效")?,
    ))
}

fn list(root: &Path) -> Result<Vec<DesignTaskBinding>> {
    let directory = binding_directory(root, false)?;
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut bindings = Vec::new();
    for entry in fs::read_dir(directory)?
        .filter_map(|entry| entry.ok())
        .take(200)
    {
        let path = entry.path();
        let record = fs::metadata(&path)
            .ok()
            .filter(|metadata| metadata.is_file() && metadata.len() <= MAX_RECORD_BYTES)
            .and_then(|_| fs::read(&path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        if let Some(record) = record {
            bindings.push(record);
        }
    }
    Ok(bindings)
}

fn persist(root: &Path, binding: &DesignTaskBinding) -> Result<()> {
    let path = record_path(root, &binding.task_id, true)?;
    fs::write(path, serde_json::to_vec_pretty(binding)?)?;
    Ok(())
}

fn record_path(root: &Path, task_id: &str, create: bool) -> Result<PathBuf> {
    validate_task_id(task_id)?;
    let directory = binding_directory(root, create)?;
    let digest = hex::encode(Sha256::digest(task_id.as_bytes()));
    Ok(directory.join(format!("{digest}.json")))
}

fn binding_directory(root: &Path, create: bool) -> Result<PathBuf> {
    let directory = root.join(".elon/ui-tuner/headless-design/task-bindings");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.exists() {
        return Ok(directory);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("设计任务绑定目录越出项目");
    }
    Ok(canonical)
}

fn required_task_id(arguments: &Value) -> Result<&str> {
    let task_id = required_text(arguments, "taskId")?;
    validate_task_id(task_id)?;
    Ok(task_id)
}

pub(super) fn validate_task_id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "._:-".contains(ch))
    {
        bail!("taskId 必须为 1..160 位 ASCII 字母、数字或 ._:-");
    }
    Ok(())
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    optional_text(value, key).ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}

fn optional_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn lease_seconds(arguments: &Value) -> Result<i64> {
    let value = arguments
        .get("leaseSeconds")
        .and_then(Value::as_i64)
        .unwrap_or(DEFAULT_LEASE_SECONDS);
    if !(60..=3600).contains(&value) {
        bail!("leaseSeconds 必须在 60..3600 之间");
    }
    Ok(value)
}

impl DesignTaskBinding {
    fn is_active(&self) -> bool {
        self.status == "ACTIVE"
            && DateTime::parse_from_rfc3339(&self.expires_at)
                .is_ok_and(|expires| expires > Utc::now())
    }
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("设计任务绑定需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}

fn task_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["taskId"],"properties":{
        "taskId":{"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"}}})
}

fn lease_schema(settle: bool) -> Value {
    let mut properties = json!({"taskId":{"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"},
        "leaseId":{"type":"string","pattern":"^lease_[a-f0-9]{32}$"},
        "leaseSeconds":{"type":"integer","minimum":60,"maximum":3600}});
    if settle {
        properties["succeeded"] = json!({"type":"boolean"});
    }
    json!({"type":"object","additionalProperties":false,"required":["taskId","leaseId"],"properties":properties})
}

fn bind_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["taskId","designSessionId"],"properties":{
        "taskId":{"type":"string","minLength":1,"maxLength":160,"pattern":"^[A-Za-z0-9._:-]+$"},
        "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
        "draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},
        "expectedLeaseId":{"type":"string","pattern":"^lease_[a-f0-9]{32}$"},
        "leaseSeconds":{"type":"integer","minimum":60,"maximum":3600}}})
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{
        "readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}
