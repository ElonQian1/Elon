use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::broker::LiveUiSession;

const TOOL: &str = "ui_check_design_source_binding";
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BindingHealth {
    pub(super) status: String,
    pub(super) ready_for_writeback: bool,
    source_file: Option<String>,
    expected_source_revision: Option<String>,
    current_source_revision: Option<String>,
    range_valid: bool,
    reason: String,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![json!({
        "name":TOOL,
        "description":"重新读取绑定源码并校验文件、SHA-256 和 byte range；漂移时可返回有界恢复候选，但不会自动改成 BOUND。",
        "inputSchema":{"type":"object","additionalProperties":false,"required":["draftId"],"properties":{
            "draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},
            "includeRecoveryCandidates":{"type":"boolean","default":true},
            "limit":{"type":"integer","minimum":1,"maximum":20,"default":8}
        }},
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}
    })]
}

pub(super) fn is_tool(name: &str) -> bool {
    name == TOOL
}

pub(super) fn call(session: &LiveUiSession, arguments: Value) -> Result<Value> {
    let draft_id = required_text(&arguments, "draftId")?;
    let draft_result =
        super::design_drafts::call(session, "ui_get_design_draft", json!({"draftId":draft_id}))?;
    let draft = draft_result
        .get("draft")
        .context("设计草稿响应缺少 draft")?;
    let health = evaluate_draft(session, draft)?;
    let include = arguments
        .get("includeRecoveryCandidates")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(8)
        .clamp(1, 20);
    let mut candidates = Vec::new();
    let mut recovery_error = None;
    if include && !health.ready_for_writeback {
        match super::design_source_binding::call(session, json!({"draftId":draft_id,"limit":limit}))
        {
            Ok(result) => {
                candidates = result
                    .get("candidates")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
            }
            Err(error) => recovery_error = Some(error.to_string()),
        }
    }
    Ok(json!({
        "schema":"elon.ui-design-binding-health.v1",
        "draftId":draft_id,
        "health":health,
        "recovery":{"candidates":candidates,"error":recovery_error,"autoRebound":false},
        "sourceModified":false,"contentEmbedded":false
    }))
}

pub(super) fn evaluate_draft(session: &LiveUiSession, draft: &Value) -> Result<BindingHealth> {
    let Some(binding) = draft.get("sourceBinding").filter(|value| !value.is_null()) else {
        return Ok(health(
            "UNBOUND",
            false,
            None,
            None,
            None,
            false,
            "草稿尚未建立 source binding",
        ));
    };
    let source_file = binding
        .get("sourceFile")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("sourceBinding 缺少 sourceFile")?;
    let expected = binding
        .get("sourceRevision")
        .and_then(Value::as_str)
        .map(str::to_string);
    if binding.get("status").and_then(Value::as_str) != Some("BOUND") {
        return Ok(health(
            "UNCONFIRMED",
            false,
            Some(source_file),
            expected.as_deref(),
            None,
            false,
            "source binding 尚未显式确认 BOUND",
        ));
    }
    let root = canonical_root(session)?;
    let path = match safe_source_path(&root, source_file) {
        Ok(path) => path,
        Err(_) => {
            return Ok(health(
                "FILE_MISSING",
                false,
                Some(source_file),
                expected.as_deref(),
                None,
                false,
                "绑定源码不存在、越出项目或不是普通文件",
            ))
        }
    };
    let metadata = fs::metadata(&path)?;
    if metadata.len() > MAX_SOURCE_BYTES {
        return Ok(health(
            "FILE_TOO_LARGE",
            false,
            Some(source_file),
            expected.as_deref(),
            None,
            false,
            "绑定源码超过健康检查大小上限",
        ));
    }
    let bytes = fs::read(path)?;
    let actual = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
    let range_valid = binding
        .get("range")
        .filter(|value| !value.is_null())
        .is_none_or(|range| {
            let start = range.get("start").and_then(Value::as_u64);
            let end = range.get("end").and_then(Value::as_u64);
            start
                .zip(end)
                .is_some_and(|(start, end)| start < end && end <= bytes.len() as u64)
        });
    if !range_valid {
        return Ok(health(
            "RANGE_STALE",
            false,
            Some(source_file),
            expected.as_deref(),
            Some(&actual),
            false,
            "绑定 byte range 已超过当前文件边界",
        ));
    }
    if expected.as_deref() != Some(actual.as_str()) {
        return Ok(health(
            "SOURCE_CHANGED",
            false,
            Some(source_file),
            expected.as_deref(),
            Some(&actual),
            true,
            "绑定后源码 SHA-256 已变化，需要重新审查候选",
        ));
    }
    Ok(health(
        "HEALTHY",
        true,
        Some(source_file),
        expected.as_deref(),
        Some(&actual),
        true,
        "源码文件、SHA-256 和 byte range 与已确认绑定一致",
    ))
}

fn health(
    status: &str,
    ready: bool,
    file: Option<&str>,
    expected: Option<&str>,
    actual: Option<&str>,
    range_valid: bool,
    reason: &str,
) -> BindingHealth {
    BindingHealth {
        status: status.to_string(),
        ready_for_writeback: ready,
        source_file: file.map(str::to_string),
        expected_source_revision: expected.map(str::to_string),
        current_source_revision: actual.map(str::to_string),
        range_valid,
        reason: reason.to_string(),
    }
}

fn safe_source_path(root: &Path, value: &str) -> Result<PathBuf> {
    let relative = Path::new(value);
    if relative.is_absolute() || relative.components().any(|part| part.as_os_str() == "..") {
        bail!("sourceFile 不是安全相对路径");
    }
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root) || !path.is_file() {
        bail!("sourceFile 越出项目或不是文件");
    }
    Ok(path)
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
            .context("源码绑定健康检查需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}
