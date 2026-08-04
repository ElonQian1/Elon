use std::{fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::{
    broker::LiveUiSession,
    design_source_patch_store::{self as store, SourcePatchProposal},
};

const PROPOSE_TOOL: &str = "ui_propose_design_source_patch";
const GET_TOOL: &str = "ui_get_design_source_patch";
const DECIDE_TOOL: &str = "ui_decide_design_source_patch";
const APPLY_TOOL: &str = "ui_apply_design_source_patch";
const ROLLBACK_TOOL: &str = "ui_plan_design_source_rollback";

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            PROPOSE_TOOL,
            "基于已批准写回计划和精确 byte range/SHA-256 生成确定性源码补丁提案；只保存审查产物，不修改源码。",
            propose_schema(),
            false,
        ),
        tool(
            GET_TOOL,
            "读取源码补丁提案的哈希、范围、审批和应用状态；源码正文保留在本地 review artifact。",
            proposal_schema(false, false),
            true,
        ),
        tool(
            DECIDE_TOOL,
            "以 expectedRevision 明确批准或拒绝源码补丁内容；批准前重新校验写回计划、草稿和源码 SHA。",
            proposal_schema(true, true),
            false,
        ),
        tool(
            APPLY_TOOL,
            "仅应用已批准且未漂移的确定性补丁；以 APPLYING journal 和原子文件替换支持幂等恢复。",
            proposal_schema(false, true),
            false,
        ),
        tool(
            ROLLBACK_TOOL,
            "为已应用补丁生成可审查的精确逆向编辑计划；只规划，不自动回滚源码。",
            proposal_schema(false, true),
            false,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(
        name,
        PROPOSE_TOOL | GET_TOOL | DECIDE_TOOL | APPLY_TOOL | ROLLBACK_TOOL
    )
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    let root = canonical_root(session)?;
    match name {
        PROPOSE_TOOL => propose(session, &root, &arguments),
        GET_TOOL => get(&root, required_text(&arguments, "proposalId")?),
        DECIDE_TOOL => decide(session, &root, &arguments),
        APPLY_TOOL => apply(session, &root, &arguments),
        ROLLBACK_TOOL => plan_rollback(&root, &arguments),
        _ => bail!("未知设计源码补丁工具: {name}"),
    }
}

fn propose(session: &LiveUiSession, root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let draft_id = required_text(arguments, "draftId")?;
    let draft_revision = required_u64(arguments, "expectedDraftRevision")?;
    let writeback_plan_id = required_text(arguments, "writebackPlanId")?;
    super::design_writeback_plan::validate_approved_plan(
        session,
        writeback_plan_id,
        draft_id,
        draft_revision,
    )?;
    let source_file = required_text(arguments, "sourceFile")?;
    validate_draft_binding(session, draft_id, draft_revision, source_file)?;
    let path = store::source_path(root, source_file)?;
    let source = fs::read(&path)?;
    std::str::from_utf8(&source).context("确定性补丁只支持 UTF-8 源码")?;
    let source_sha_before = store::source_sha(&source);
    let edits = store::build_edits(&source, arguments.get("edits").context("缺少 edits")?)?;
    let output = store::apply_edits(&source, &edits)?;
    let source_sha_after = store::source_sha(&output);
    if source_sha_after == source_sha_before {
        bail!("SOURCE_PATCH_NO_CHANGE：补丁没有改变源码");
    }
    let proposal_id = store::proposal_id(
        writeback_plan_id,
        draft_revision,
        &source_sha_before,
        &edits,
    )?;
    if let Ok(existing) = store::read(root, &proposal_id) {
        return Ok(response("UNCHANGED", &existing, false));
    }
    let review_artifact_path =
        store::write_review_artifact(root, &proposal_id, source_file, &edits)?;
    let now = chrono::Utc::now().to_rfc3339();
    let proposal = SourcePatchProposal {
        schema_version: 1,
        proposal_id,
        revision: 1,
        writeback_plan_id: writeback_plan_id.to_string(),
        draft_id: draft_id.to_string(),
        draft_revision,
        source_file: source_file.to_string(),
        source_sha_before,
        source_sha_after,
        edits,
        status: "PROPOSED".to_string(),
        decision_reason: None,
        review_artifact_path,
        created_at: now.clone(),
        updated_at: now,
        applied_at: None,
    };
    store::persist(root, &proposal)?;
    Ok(response("PROPOSED", &proposal, false))
}

fn get(root: &std::path::Path, proposal_id: &str) -> Result<Value> {
    let proposal = store::read(root, proposal_id)?;
    Ok(response("READ", &proposal, false))
}

fn decide(session: &LiveUiSession, root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let mut proposal = load_expected(root, arguments)?;
    if proposal.status != "PROPOSED" {
        bail!("SOURCE_PATCH_STATE_CONFLICT：只有 PROPOSED 提案可以审批");
    }
    let decision = required_text(arguments, "decision")?.to_ascii_uppercase();
    if !matches!(decision.as_str(), "APPROVE" | "REJECT") {
        bail!("decision 只允许 APPROVE 或 REJECT");
    }
    let reason = optional_text(arguments, "reason").map(|value| clean(value, 1_000));
    if decision == "REJECT" && reason.is_none() {
        bail!("拒绝源码补丁必须提供 reason");
    }
    if decision == "APPROVE" {
        validate_contract(session, root, &proposal)?;
    }
    proposal.revision += 1;
    proposal.status = if decision == "APPROVE" {
        "APPROVED"
    } else {
        "REJECTED"
    }
    .to_string();
    proposal.decision_reason = reason;
    proposal.updated_at = chrono::Utc::now().to_rfc3339();
    store::persist(root, &proposal)?;
    Ok(response(proposal.status.as_str(), &proposal, false))
}

fn apply(session: &LiveUiSession, root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let mut proposal = load_expected(root, arguments)?;
    if proposal.status == "APPLIED" {
        return Ok(response("UNCHANGED", &proposal, true));
    }
    if !matches!(proposal.status.as_str(), "APPROVED" | "APPLYING") {
        bail!("SOURCE_PATCH_NOT_APPROVED：补丁未批准或已拒绝");
    }
    let path = store::source_path(root, &proposal.source_file)?;
    let mut source = fs::read(&path)?;
    let actual_sha = store::source_sha(&source);
    if proposal.status == "APPROVED" {
        validate_contract(session, root, &proposal)?;
        proposal.status = "APPLYING".to_string();
        proposal.revision += 1;
        proposal.updated_at = chrono::Utc::now().to_rfc3339();
        store::persist(root, &proposal)?;
    }
    if actual_sha == proposal.source_sha_before {
        source = store::apply_edits(&source, &proposal.edits)?;
        if store::source_sha(&source) != proposal.source_sha_after {
            bail!("SOURCE_PATCH_OUTPUT_MISMATCH：输出 SHA 与提案不一致");
        }
        crate::node_agent_atomic_file::write(&path, &source)?;
    } else if actual_sha != proposal.source_sha_after {
        bail!("SOURCE_PATCH_RECOVERY_REQUIRED：源码既不是应用前也不是应用后 revision");
    }
    let now = chrono::Utc::now().to_rfc3339();
    proposal.status = "APPLIED".to_string();
    proposal.revision += 1;
    proposal.updated_at = now.clone();
    proposal.applied_at = Some(now);
    store::persist(root, &proposal)?;
    Ok(response("APPLIED", &proposal, true))
}

fn plan_rollback(root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let proposal = load_expected(root, arguments)?;
    if proposal.status != "APPLIED" {
        bail!("SOURCE_PATCH_NOT_APPLIED：只有已应用补丁可以生成回滚计划");
    }
    let path = store::source_path(root, &proposal.source_file)?;
    if store::source_sha(&fs::read(path)?) != proposal.source_sha_after {
        bail!("SOURCE_ROLLBACK_DRIFT：应用后源码已变化，不能生成精确回滚计划");
    }
    let rollback = store::plan_rollback(root, &proposal)?;
    Ok(json!({
        "schema":"elon.ui-design-source-rollback-plan.v1","action":"PLANNED",
        "rollback":rollback,"sourceModified":false
    }))
}

fn validate_contract(
    session: &LiveUiSession,
    root: &std::path::Path,
    proposal: &SourcePatchProposal,
) -> Result<()> {
    super::design_writeback_plan::validate_approved_plan(
        session,
        &proposal.writeback_plan_id,
        &proposal.draft_id,
        proposal.draft_revision,
    )?;
    validate_draft_binding(
        session,
        &proposal.draft_id,
        proposal.draft_revision,
        &proposal.source_file,
    )?;
    let path = store::source_path(root, &proposal.source_file)?;
    if store::source_sha(&fs::read(path)?) != proposal.source_sha_before {
        bail!("SOURCE_PATCH_SOURCE_DRIFT：源码已偏离提案基线");
    }
    Ok(())
}

fn validate_draft_binding(
    session: &LiveUiSession,
    draft_id: &str,
    draft_revision: u64,
    source_file: &str,
) -> Result<()> {
    let result =
        super::design_drafts::call(session, "ui_get_design_draft", json!({"draftId":draft_id}))?;
    let draft = result.get("draft").context("设计草稿响应缺少 draft")?;
    if draft.get("revision").and_then(Value::as_u64) != Some(draft_revision) {
        bail!("SOURCE_PATCH_DRAFT_STALE：草稿 revision 已变化");
    }
    if draft
        .pointer("/sourceBinding/status")
        .and_then(Value::as_str)
        != Some("BOUND")
        || draft
            .pointer("/sourceBinding/sourceFile")
            .and_then(Value::as_str)
            != Some(source_file)
    {
        bail!("SOURCE_PATCH_BINDING_MISMATCH：sourceFile 不是草稿已确认绑定");
    }
    Ok(())
}

fn load_expected(root: &std::path::Path, arguments: &Value) -> Result<SourcePatchProposal> {
    let proposal_id = required_text(arguments, "proposalId")?;
    let expected = required_u64(arguments, "expectedRevision")?;
    let proposal = store::read(root, proposal_id)?;
    if proposal.revision != expected {
        bail!(
            "SOURCE_PATCH_REVISION_CONFLICT：expected={expected} actual={}",
            proposal.revision
        );
    }
    Ok(proposal)
}

fn response(action: &str, proposal: &SourcePatchProposal, source_modified: bool) -> Value {
    json!({
        "schema":"elon.ui-design-source-patch.v1","action":action,
        "proposal":store::proposal_view(proposal),"sourceModified":source_modified
    })
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("设计源码补丁需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
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

fn required_u64(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}

fn clean(value: &str, max: usize) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| *ch != '\0')
        .take(max)
        .collect()
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{
        "readOnlyHint":read_only,"destructiveHint":name == APPLY_TOOL,
        "idempotentHint":name != PROPOSE_TOOL,"openWorldHint":false}})
}

fn proposal_reference() -> Value {
    json!({
        "proposalId":{"type":"string","pattern":"^sourcepatch_[a-f0-9]{32}$"},
        "expectedRevision":{"type":"integer","minimum":1}
    })
}

fn propose_schema() -> Value {
    json!({
        "type":"object","additionalProperties":false,
        "required":["writebackPlanId","draftId","expectedDraftRevision","sourceFile","edits"],
        "properties":{
            "writebackPlanId":{"type":"string","pattern":"^writeplan_[a-f0-9]{32}$"},
            "draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},
            "expectedDraftRevision":{"type":"integer","minimum":1},
            "sourceFile":{"type":"string","minLength":1,"maxLength":1000},
            "edits":{"type":"array","minItems":1,"maxItems":16,"items":{
                "type":"object","additionalProperties":false,
                "required":["start","end","expectedBeforeSha256","replacement"],
                "properties":{"start":{"type":"integer","minimum":0},"end":{"type":"integer","minimum":0},
                    "expectedBeforeSha256":{"type":"string","pattern":"^sha256:[a-f0-9]{64}$"},
                    "replacement":{"type":"string","maxLength":65536}}
            }}
        }
    })
}

fn proposal_schema(decide: bool, require_revision: bool) -> Value {
    let mut properties = proposal_reference();
    let mut required = vec!["proposalId"];
    if decide {
        properties["decision"] = json!({"enum":["APPROVE","REJECT"]});
        properties["reason"] = json!({"type":"string","maxLength":1000});
    }
    if require_revision {
        required.push("expectedRevision");
    }
    if decide {
        required.push("decision");
    }
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}
