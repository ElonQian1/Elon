use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{broker::LiveUiSession, design_binding_health};

const PLAN_TOOL: &str = "ui_plan_design_writeback";
const GET_TOOL: &str = "ui_get_design_writeback_plan";
const DECIDE_TOOL: &str = "ui_decide_design_writeback_plan";
const MAX_PLAN_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WritebackPlan {
    schema_version: u32,
    plan_id: String,
    plan_revision: u64,
    draft_id: String,
    draft_revision: u64,
    design_session_id: String,
    source_file: Option<String>,
    expected_source_revision: Option<String>,
    binding_health: String,
    target_platforms: Vec<String>,
    operation_count: usize,
    items: Vec<WritebackItem>,
    impact: Value,
    decision: String,
    decision_reason: Option<String>,
    decided_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WritebackItem {
    operation_index: usize,
    operation_type: String,
    platform: String,
    adapter: String,
    mutation_kind: String,
    readiness: String,
    deterministic: bool,
    source_file: Option<String>,
    range: Option<Value>,
    reason: String,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(PLAN_TOOL, "把当前 DraftOperation、平台能力、源码绑定健康和影响范围编译为持久写回计划；只规划，不修改源码。", draft_schema(), false),
        tool(GET_TOOL, "读取写回计划的操作、适配器、风险和审批状态。", plan_schema(false), true),
        tool(DECIDE_TOOL, "以 expectedPlanRevision 显式批准或拒绝写回计划；批准只授权进入写回基线，不直接改源码。", plan_schema(true), false),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(name, PLAN_TOOL | GET_TOOL | DECIDE_TOOL)
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    match name {
        PLAN_TOOL => plan(session, required_text(&arguments, "draftId")?),
        GET_TOOL => get_response(session, required_text(&arguments, "planId")?),
        DECIDE_TOOL => decide(session, &arguments),
        _ => bail!("未知设计写回计划工具: {name}"),
    }
}

pub(super) fn validate_approved_plan(
    session: &LiveUiSession,
    plan_id: &str,
    draft_id: &str,
    draft_revision: u64,
) -> Result<()> {
    let root = canonical_root(session)?;
    let plan = read(&root, plan_id)?;
    if plan.draft_id != draft_id || plan.draft_revision != draft_revision {
        bail!("DESIGN_WRITEBACK_PLAN_STALE：计划不属于当前 draft revision");
    }
    if plan.decision != "APPROVED" {
        bail!("DESIGN_WRITEBACK_PLAN_NOT_APPROVED：写回计划尚未批准");
    }
    let draft = get_draft(session, draft_id)?;
    let health = design_binding_health::evaluate_draft(session, &draft)?;
    if !health.ready_for_writeback {
        bail!("DESIGN_SOURCE_BINDING_DRIFT：批准后源码绑定已漂移");
    }
    Ok(())
}

fn plan(session: &LiveUiSession, draft_id: &str) -> Result<Value> {
    let root = canonical_root(session)?;
    let draft = get_draft(session, draft_id)?;
    let draft_revision = draft
        .get("revision")
        .and_then(Value::as_u64)
        .context("草稿缺少 revision")?;
    let source_binding = draft.get("sourceBinding").filter(|value| !value.is_null());
    let source_file = source_binding
        .and_then(|value| value.get("sourceFile"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let source_revision = source_binding
        .and_then(|value| value.get("sourceRevision"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let plan_id = deterministic_plan_id(draft_id, draft_revision, source_revision.as_deref());
    if let Ok(existing) = read(&root, &plan_id) {
        return Ok(
            json!({"schema":"elon.ui-design-writeback-plan.v1","plan":existing,"action":"UNCHANGED","sourceModified":false}),
        );
    }
    let health = design_binding_health::evaluate_draft(session, &draft)?;
    let platforms = strings(draft.get("targetPlatforms"));
    let operations = operation_views(&draft);
    let range = source_binding
        .and_then(|value| value.get("range"))
        .filter(|value| !value.is_null())
        .cloned();
    let mut items = Vec::new();
    for (index, operation) in operations.iter().enumerate() {
        let operation_type = operation
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("UNKNOWN");
        for platform in &platforms {
            let capability = capability_status(&draft, index, platform);
            let adapter = adapter_for(source_file.as_deref(), platform, operation_type);
            let readiness = if capability == "UNSUPPORTED" {
                "UNSUPPORTED"
            } else if !health.ready_for_writeback {
                "BLOCKED_BINDING"
            } else if adapter == "GENERIC_AI_SOURCE_HANDOFF" {
                "READY_FOR_AI_HANDOFF"
            } else {
                "READY_FOR_REVIEW"
            };
            let deterministic = matches!(adapter, "CSS_RULE_ADAPTER" | "ANDROID_XML_ADAPTER")
                && operation_type == "SET_STYLE";
            items.push(WritebackItem {
                operation_index: index,
                operation_type: operation_type.to_string(),
                platform: platform.clone(),
                adapter: adapter.to_string(),
                mutation_kind: mutation_kind(operation_type).to_string(),
                readiness: readiness.to_string(),
                deterministic,
                source_file: source_file.clone(),
                range: range.clone(),
                reason: readiness_reason(readiness, capability, &health.status).to_string(),
            });
        }
    }
    let risk = risk_level(&operations);
    let blocked = items
        .iter()
        .filter(|item| {
            !matches!(
                item.readiness.as_str(),
                "READY_FOR_REVIEW" | "READY_FOR_AI_HANDOFF"
            )
        })
        .count();
    let now = chrono::Utc::now().to_rfc3339();
    let record = WritebackPlan {
        schema_version: 1,
        plan_id,
        plan_revision: 1,
        draft_id: draft_id.to_string(),
        draft_revision,
        design_session_id: required_text(&draft, "designSessionId")?.to_string(),
        source_file: source_file.clone(),
        expected_source_revision: source_revision,
        binding_health: health.status,
        target_platforms: platforms,
        operation_count: operations.len(),
        items,
        impact: json!({
            "riskLevel":risk,"requiresExplicitApproval":true,
            "structuralChange":operations.iter().any(|value| matches!(value.get("type").and_then(Value::as_str), Some("INSERT_NODE" | "REMOVE_NODE" | "MOVE_NODE"))),
            "assetChange":operations.iter().any(|value| value.get("type").and_then(Value::as_str) == Some("REPLACE_ASSET")),
            "files":source_file.into_iter().collect::<Vec<_>>(),"blockedItemCount":blocked,
            "sourceDiffAvailable":false,"runtimeVerificationRequired":true
        }),
        decision: "PROPOSED".to_string(),
        decision_reason: None,
        decided_at: None,
        created_at: now.clone(),
        updated_at: now,
    };
    persist(&root, &record)?;
    Ok(
        json!({"schema":"elon.ui-design-writeback-plan.v1","plan":record,"action":"PLANNED","sourceModified":false}),
    )
}

fn decide(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_root(session)?;
    let plan_id = required_text(arguments, "planId")?;
    let expected = arguments
        .get("expectedPlanRevision")
        .and_then(Value::as_u64)
        .context("缺少 expectedPlanRevision")?;
    let decision = required_text(arguments, "decision")?.to_ascii_uppercase();
    if !matches!(decision.as_str(), "APPROVE" | "REJECT") {
        bail!("decision 只允许 APPROVE 或 REJECT");
    }
    let reason = arguments
        .get("reason")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if decision == "REJECT" && reason.is_none() {
        bail!("拒绝写回计划必须提供 reason");
    }
    if reason.is_some_and(|value| value.chars().count() > 1_000 || value.contains('\0')) {
        bail!("reason 过长或包含 NUL");
    }
    let mut plan = read(&root, plan_id)?;
    if plan.plan_revision != expected {
        bail!(
            "DESIGN_WRITEBACK_PLAN_CONFLICT：expected={expected} actual={}",
            plan.plan_revision
        );
    }
    if decision == "APPROVE" {
        let draft = get_draft(session, &plan.draft_id)?;
        if draft.get("revision").and_then(Value::as_u64) != Some(plan.draft_revision) {
            bail!("DESIGN_WRITEBACK_PLAN_STALE：草稿 revision 已变化");
        }
        let health = design_binding_health::evaluate_draft(session, &draft)?;
        if !health.ready_for_writeback {
            bail!("DESIGN_SOURCE_BINDING_DRIFT：源码绑定不健康，不能批准");
        }
        if plan.items.iter().any(|item| {
            !matches!(
                item.readiness.as_str(),
                "READY_FOR_REVIEW" | "READY_FOR_AI_HANDOFF"
            )
        }) {
            bail!("DESIGN_WRITEBACK_PLAN_BLOCKED：仍有未就绪计划项");
        }
    }
    let now = chrono::Utc::now().to_rfc3339();
    plan.plan_revision += 1;
    plan.decision = if decision == "APPROVE" {
        "APPROVED"
    } else {
        "REJECTED"
    }
    .to_string();
    plan.decision_reason = reason.map(str::to_string);
    plan.decided_at = Some(now.clone());
    plan.updated_at = now;
    persist(&root, &plan)?;
    Ok(
        json!({"schema":"elon.ui-design-writeback-plan.v1","action":plan.decision,"plan":plan,"sourceModified":false}),
    )
}

fn get_response(session: &LiveUiSession, plan_id: &str) -> Result<Value> {
    let plan = read(&canonical_root(session)?, plan_id)?;
    Ok(json!({"schema":"elon.ui-design-writeback-plan.v1","plan":plan,"contentEmbedded":false}))
}

fn get_draft(session: &LiveUiSession, draft_id: &str) -> Result<Value> {
    let result =
        super::design_drafts::call(session, "ui_get_design_draft", json!({"draftId":draft_id}))?;
    result
        .get("draft")
        .cloned()
        .context("设计草稿响应缺少 draft")
}

fn operation_views(draft: &Value) -> Vec<Value> {
    let operations = draft
        .get("operations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !operations.is_empty() {
        return operations;
    }
    draft
        .get("patches")
        .and_then(Value::as_array)
        .map(|patches| {
            patches.iter().map(|patch| json!({
        "type":"SET_STYLE","property":patch.get("property"),"after":patch.get("after")
    })).collect()
        })
        .unwrap_or_default()
}

fn capability_status<'a>(draft: &'a Value, index: usize, platform: &str) -> &'a str {
    draft
        .pointer("/operationCapabilities/entries")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().find(|entry| {
                entry.get("operationIndex").and_then(Value::as_u64) == Some(index as u64)
                    && entry.get("platform").and_then(Value::as_str) == Some(platform)
            })
        })
        .and_then(|entry| entry.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("SOURCE_HANDOFF")
}

fn adapter_for(file: Option<&str>, platform: &str, operation: &str) -> &'static str {
    let extension = file
        .and_then(|file| Path::new(file).extension())
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match (extension.as_str(), platform, operation) {
        ("css" | "scss" | "less", _, "SET_STYLE" | "SET_RESPONSIVE_STYLE") => "CSS_RULE_ADAPTER",
        ("tsx" | "jsx" | "ts" | "js", "web" | "pwa" | "tauri", _) => "REACT_TYPESCRIPT_ADAPTER",
        ("vue", "web" | "pwa" | "tauri", _) => "VUE_SFC_ADAPTER",
        ("svelte", "web" | "pwa" | "tauri", _) => "SVELTE_COMPONENT_ADAPTER",
        ("kt" | "kts", "android", _) => "ANDROID_COMPOSE_ADAPTER",
        ("xml", "android", _) => "ANDROID_XML_ADAPTER",
        _ => "GENERIC_AI_SOURCE_HANDOFF",
    }
}

fn mutation_kind(operation: &str) -> &'static str {
    match operation {
        "SET_STYLE" | "SET_RESPONSIVE_STYLE" => "STYLE_UPDATE",
        "SET_TEXT" => "TEXT_UPDATE",
        "REPLACE_ASSET" => "ASSET_REFERENCE_UPDATE",
        "SET_VARIANT" => "VARIANT_UPDATE",
        "INSERT_NODE" => "STRUCTURE_INSERT",
        "REMOVE_NODE" => "STRUCTURE_REMOVE",
        "MOVE_NODE" => "STRUCTURE_MOVE",
        _ => "UNKNOWN",
    }
}
fn readiness_reason(readiness: &str, capability: &str, _health: &str) -> &'static str {
    match readiness {
        "UNSUPPORTED" => "目标平台声明该操作不受支持",
        "BLOCKED_BINDING" => "源码绑定健康检查未通过",
        "READY_FOR_AI_HANDOFF" => "尚无匹配框架的确定性适配器，批准后交给受审 AI handoff",
        _ if capability == "LIVE_PREVIEW" => "已可预览，源码写回仍需显式批准",
        _ => "绑定和平台适配器已具备审查条件",
    }
}
fn risk_level(operations: &[Value]) -> &'static str {
    if operations.iter().any(|value| {
        matches!(
            value.get("type").and_then(Value::as_str),
            Some("INSERT_NODE" | "REMOVE_NODE" | "MOVE_NODE")
        )
    }) {
        "HIGH"
    } else if operations.iter().any(|value| {
        matches!(
            value.get("type").and_then(Value::as_str),
            Some("REPLACE_ASSET" | "SET_TEXT" | "SET_VARIANT" | "SET_RESPONSIVE_STYLE")
        )
    }) {
        "MEDIUM"
    } else {
        "LOW"
    }
}
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn deterministic_plan_id(draft_id: &str, revision: u64, source_revision: Option<&str>) -> String {
    let digest = hex::encode(Sha256::digest(
        format!("{draft_id}\0{revision}\0{}", source_revision.unwrap_or("")).as_bytes(),
    ));
    format!("writeplan_{}", &digest[..32])
}
fn persist(root: &Path, plan: &WritebackPlan) -> Result<()> {
    let directory = plan_directory(root, true)?;
    fs::write(
        directory.join(format!("{}.json", plan.plan_id)),
        serde_json::to_vec_pretty(plan)?,
    )?;
    Ok(())
}
fn read(root: &Path, plan_id: &str) -> Result<WritebackPlan> {
    validate_plan_id(plan_id)?;
    let path = plan_directory(root, false)?.join(format!("{plan_id}.json"));
    let metadata = fs::metadata(&path).context("写回计划不存在")?;
    if !metadata.is_file() || metadata.len() > MAX_PLAN_BYTES {
        bail!("写回计划无效或过大");
    }
    serde_json::from_slice(&fs::read(path)?).context("写回计划 JSON 无效")
}
fn plan_directory(root: &Path, create: bool) -> Result<PathBuf> {
    let directory = root.join(".elon/ui-tuner/headless-design/writeback-plans");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.exists() {
        return Ok(directory);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("写回计划目录越出项目");
    }
    Ok(canonical)
}
fn validate_plan_id(value: &str) -> Result<()> {
    if value.len() != 42
        || !value.starts_with("writeplan_")
        || !value[10..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("planId 无效");
    }
    Ok(())
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
            .context("设计写回计划需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}
fn draft_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["draftId"],"properties":{"draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"}}})
}
fn plan_schema(decide: bool) -> Value {
    let mut properties = json!({"planId":{"type":"string","pattern":"^writeplan_[a-f0-9]{32}$"}});
    let mut required = vec!["planId"];
    if decide {
        properties["expectedPlanRevision"] = json!({"type":"integer","minimum":1});
        properties["decision"] = json!({"enum":["APPROVE","REJECT"]});
        properties["reason"] = json!({"type":"string","maxLength":1000});
        required.extend(["expectedPlanRevision", "decision"]);
    }
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}
fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{"readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":read_only,"openWorldHint":false}})
}
