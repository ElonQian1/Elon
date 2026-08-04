use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    broker::LiveUiSession,
    design_draft_operations::{self, DraftOperation},
    design_session_store::{read_record, validate_design_session_id},
};
use crate::node_agent_source_preview::{
    begin_writeback_receipt, complete_writeback_receipt, BeginWritebackReceiptRequest,
    CompleteWritebackReceiptRequest, PlatformReceiptUpdate,
};

const LIST_TOOL: &str = "ui_list_design_drafts";
const CREATE_TOOL: &str = "ui_create_design_draft";
const GET_TOOL: &str = "ui_get_design_draft";
const UPDATE_TOOL: &str = "ui_update_design_draft";
const UNDO_TOOL: &str = "ui_undo_design_draft";
const BEGIN_WRITEBACK_TOOL: &str = "ui_begin_design_writeback";
const COMPLETE_WRITEBACK_TOOL: &str = "ui_complete_design_writeback";
const MAX_HISTORY: usize = 50;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DesignStylePatch {
    property: String,
    before: Option<String>,
    after: String,
    unit: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DesignSourceBinding {
    status: String,
    source_file: String,
    symbol: Option<String>,
    kind: String,
    range: Option<SourceRange>,
    source_revision: Option<String>,
    confidence: String,
    reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceRange {
    start: u64,
    end: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DraftSnapshot {
    revision: u64,
    patches: Vec<DesignStylePatch>,
    #[serde(default)]
    operations: Vec<DraftOperation>,
    source_binding: Option<DesignSourceBinding>,
    target_platforms: Vec<String>,
    status: String,
    captured_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesignDraft {
    schema_version: u8,
    draft_id: String,
    design_session_id: String,
    platform: String,
    route: String,
    selector: String,
    scope: String,
    patches: Vec<DesignStylePatch>,
    #[serde(default)]
    operations: Vec<DraftOperation>,
    source_binding: Option<DesignSourceBinding>,
    target_platforms: Vec<String>,
    revision: u64,
    status: String,
    writeback_receipt_id: Option<String>,
    history: Vec<DraftSnapshot>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateDraftRequest {
    design_session_id: String,
    selector: String,
    #[serde(default = "default_scope")]
    scope: String,
    #[serde(default)]
    patches: Vec<DesignStylePatch>,
    #[serde(default)]
    operations: Vec<DraftOperation>,
    source_binding: Option<DesignSourceBinding>,
    #[serde(default)]
    target_platforms: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateDraftRequest {
    draft_id: String,
    expected_revision: u64,
    patches: Option<Vec<DesignStylePatch>>,
    operations: Option<Vec<DraftOperation>>,
    source_binding: Option<DesignSourceBinding>,
    target_platforms: Option<Vec<String>>,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(LIST_TOOL, "列出项目最近的通用多端设计草稿；只返回 selector、revision、平台和绑定状态，不返回历史正文。", json!({"type":"object","additionalProperties":false,"properties":{"limit":{"type":"integer","minimum":1,"maximum":50,"default":20},"designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"}}}), true),
        tool(CREATE_TOOL, "基于 designSession selector 创建项目持久的多端 DraftOperation v2 草稿；草稿只描述可撤销意图，不直接冒充源码修改。", draft_create_schema(), false),
        tool(GET_TOOL, "读取单个设计草稿、源码绑定、样式 patch、revision 和写回回执引用。", draft_id_schema(true), true),
        tool(UPDATE_TOOL, "以 expectedRevision 乐观并发更新 DraftOperation、兼容样式 patch、source binding 或目标平台；旧快照进入有界撤销历史。", draft_update_schema(), false),
        tool(UNDO_TOOL, "撤销设计草稿的最近一次更新；revision 仍单调递增，避免旧客户端覆盖新状态。", draft_id_schema(false), false),
        tool(BEGIN_WRITEBACK_TOOL, "验证已批准且未漂移的写回计划，固定当前草稿 revision 和 Git/sourceRevision，并开始分平台写回机器回执。", begin_writeback_schema(), false),
        tool(COMPLETE_WRITEBACK_TOOL, "根据实际 changedFiles、源码哈希与分平台 build evidence 更新写回回执；没有证据不会显示完成。", complete_writeback_schema(), false),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(
        name,
        LIST_TOOL
            | CREATE_TOOL
            | GET_TOOL
            | UPDATE_TOOL
            | UNDO_TOOL
            | BEGIN_WRITEBACK_TOOL
            | COMPLETE_WRITEBACK_TOOL
    )
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    match name {
        LIST_TOOL => list(session, &arguments),
        CREATE_TOOL => create(
            session,
            serde_json::from_value(arguments).context("创建设计草稿参数无效")?,
        ),
        GET_TOOL => get(session, required_draft_id(&arguments)?),
        UPDATE_TOOL => update(
            session,
            serde_json::from_value(arguments).context("更新设计草稿参数无效")?,
        ),
        UNDO_TOOL => undo(
            session,
            required_draft_id(&arguments)?,
            required_revision(&arguments)?,
        ),
        BEGIN_WRITEBACK_TOOL => begin_writeback(
            session,
            required_draft_id(&arguments)?,
            required_revision(&arguments)?,
            required_text(&arguments, "writebackPlanId")?,
        ),
        COMPLETE_WRITEBACK_TOOL => complete_writeback(session, &arguments),
        _ => bail!("未知设计草稿工具: {name}"),
    }
}

fn list(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_root(session)?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 50) as usize;
    let filter = arguments
        .get("designSessionId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(id) = filter {
        validate_design_session_id(id)?;
    }
    let directory = draft_directory(&root, false)?;
    let mut drafts = if let Some(directory) = directory {
        fs::read_dir(directory)?
            .filter_map(|entry| entry.ok())
            .take(200)
            .filter_map(|entry| read_draft_file(&entry.path()).ok())
            .filter(|draft| filter.is_none_or(|id| draft.design_session_id == id))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    drafts.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    drafts.truncate(limit);
    let summaries = drafts.into_iter().map(|draft| json!({
        "draftId":draft.draft_id,"designSessionId":draft.design_session_id,"platform":draft.platform,
        "route":draft.route,"selector":draft.selector,"scope":draft.scope,"revision":draft.revision,
        "status":draft.status,"targetPlatforms":draft.target_platforms,
        "operationCount":draft.operations.len(),
        "sourceBindingStatus":draft.source_binding.as_ref().map(|binding| binding.status.as_str()),
        "writebackReceiptId":draft.writeback_receipt_id,"updatedAt":draft.updated_at,
    })).collect::<Vec<_>>();
    Ok(json!({"schemaVersion":2,"drafts":summaries,"contentEmbedded":false}))
}

fn create(session: &LiveUiSession, mut request: CreateDraftRequest) -> Result<Value> {
    let root = canonical_root(session)?;
    validate_design_session_id(&request.design_session_id)?;
    let design_session = read_record(&root, &request.design_session_id)?;
    request.selector = clean_text(&request.selector, 1_000, "selector")?;
    request.scope = normalize_scope(&request.scope)?;
    request.patches = normalize_patches(request.patches)?;
    request.operations = design_draft_operations::normalize_operations(request.operations)?;
    request.source_binding = request.source_binding.map(normalize_binding).transpose()?;
    let fallback = vec![design_session.platform.as_str().to_string()];
    let target_platforms = normalize_platforms(if request.target_platforms.is_empty() {
        fallback
    } else {
        request.target_platforms
    })?;
    let now = chrono::Utc::now().to_rfc3339();
    let draft = DesignDraft {
        schema_version: 2,
        draft_id: format!("draft_{}", uuid::Uuid::new_v4().simple()),
        design_session_id: request.design_session_id,
        platform: design_session.platform.as_str().into(),
        route: design_session.route,
        selector: request.selector,
        scope: request.scope,
        patches: request.patches,
        operations: request.operations,
        source_binding: request.source_binding,
        target_platforms,
        revision: 1,
        status: "DRAFT".into(),
        writeback_receipt_id: None,
        history: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    };
    persist(&root, &draft)?;
    let next = if draft.source_binding.is_some() {
        "ui_plan_design_writeback"
    } else {
        UPDATE_TOOL
    };
    Ok(json!({"draft":draft_view(&draft),"next":next}))
}

fn get(session: &LiveUiSession, draft_id: &str) -> Result<Value> {
    let root = canonical_root(session)?;
    let draft = read(&root, draft_id)?;
    Ok(json!({"draft":draft_view(&draft),"contentEmbedded":false}))
}

fn update(session: &LiveUiSession, request: UpdateDraftRequest) -> Result<Value> {
    let root = canonical_root(session)?;
    let mut draft = read(&root, &request.draft_id)?;
    expect_revision(&draft, request.expected_revision)?;
    push_history(&mut draft);
    if let Some(patches) = request.patches {
        draft.patches = normalize_patches(patches)?;
    }
    if let Some(operations) = request.operations {
        draft.operations = design_draft_operations::normalize_operations(operations)?;
    }
    if let Some(binding) = request.source_binding {
        draft.source_binding = Some(normalize_binding(binding)?);
    }
    if let Some(platforms) = request.target_platforms {
        draft.target_platforms = normalize_platforms(platforms)?;
    }
    draft.revision += 1;
    draft.schema_version = 2;
    draft.status = "DRAFT".into();
    draft.writeback_receipt_id = None;
    draft.updated_at = chrono::Utc::now().to_rfc3339();
    persist(&root, &draft)?;
    Ok(json!({"draft":draft_view(&draft)}))
}

fn undo(session: &LiveUiSession, draft_id: &str, expected: u64) -> Result<Value> {
    let root = canonical_root(session)?;
    let mut draft = read(&root, draft_id)?;
    expect_revision(&draft, expected)?;
    let snapshot = draft
        .history
        .pop()
        .context("DESIGN_DRAFT_UNDO_EMPTY：没有可撤销修改")?;
    draft.patches = snapshot.patches;
    draft.operations = snapshot.operations;
    draft.source_binding = snapshot.source_binding;
    draft.target_platforms = snapshot.target_platforms;
    draft.status = "DRAFT".into();
    draft.writeback_receipt_id = None;
    draft.revision += 1;
    draft.schema_version = 2;
    draft.updated_at = chrono::Utc::now().to_rfc3339();
    persist(&root, &draft)?;
    Ok(json!({"draft":draft_view(&draft),"undidRevision":snapshot.revision}))
}

fn begin_writeback(
    session: &LiveUiSession,
    draft_id: &str,
    expected: u64,
    writeback_plan_id: &str,
) -> Result<Value> {
    let root = canonical_root(session)?;
    let mut draft = read(&root, draft_id)?;
    expect_revision(&draft, expected)?;
    if draft.patches.is_empty() && draft.operations.is_empty() {
        bail!("DESIGN_DRAFT_EMPTY：草稿没有 patch 或 DraftOperation");
    }
    let binding = draft
        .source_binding
        .as_ref()
        .context("DESIGN_SOURCE_BINDING_REQUIRED：写回前必须建立 source binding")?;
    if binding.status != "BOUND" {
        bail!("DESIGN_SOURCE_BINDING_UNCONFIRMED：写回前 binding.status 必须是 BOUND");
    }
    validate_binding_target(&root, binding)?;
    super::design_writeback_plan::validate_approved_plan(
        session,
        writeback_plan_id,
        draft_id,
        expected,
    )?;
    let receipt = begin_writeback_receipt(BeginWritebackReceiptRequest {
        operation_id: format!(
            "design-draft:{}:r{}:{}",
            draft.draft_id, draft.revision, writeback_plan_id
        ),
        project_root: root.to_string_lossy().to_string(),
        draft_revision: draft.revision,
        target_platforms: draft.target_platforms.clone(),
    })?;
    draft.status = "WRITEBACK_PREVIEW".into();
    draft.writeback_receipt_id = Some(receipt.receipt_id.clone());
    draft.updated_at = chrono::Utc::now().to_rfc3339();
    persist(&root, &draft)?;
    Ok(
        json!({"draft":draft_view(&draft),"writebackPlanId":writeback_plan_id,"receipt":receipt,"next":COMPLETE_WRITEBACK_TOOL}),
    )
}

fn complete_writeback(session: &LiveUiSession, arguments: &Value) -> Result<Value> {
    let root = canonical_root(session)?;
    let draft_id = required_draft_id(arguments)?;
    let expected = required_revision(arguments)?;
    let mut draft = read(&root, draft_id)?;
    expect_revision(&draft, expected)?;
    let receipt_id = draft
        .writeback_receipt_id
        .clone()
        .context("DESIGN_WRITEBACK_NOT_STARTED：先 begin")?;
    let platform_results = serde_json::from_value::<BTreeMap<String, PlatformReceiptUpdate>>(
        arguments
            .get("platformResults")
            .cloned()
            .context("缺少 platformResults")?,
    )
    .context("platformResults 不符合写回回执契约")?;
    let receipt = complete_writeback_receipt(CompleteWritebackReceiptRequest {
        receipt_id,
        project_root: root.to_string_lossy().to_string(),
        platform_results,
    })?;
    draft.status = if receipt.complete {
        "COMPLETE"
    } else if receipt.status == "FAILED" {
        "FAILED"
    } else {
        "WRITEBACK_IN_PROGRESS"
    }
    .into();
    draft.updated_at = chrono::Utc::now().to_rfc3339();
    persist(&root, &draft)?;
    Ok(json!({"draft":draft_view(&draft),"receipt":receipt}))
}

fn draft_view(draft: &DesignDraft) -> Value {
    json!({
        "schemaVersion":draft.schema_version,"draftId":draft.draft_id,
        "designSessionId":draft.design_session_id,"platform":draft.platform,"route":draft.route,
        "selector":draft.selector,"scope":draft.scope,"patches":draft.patches,
        "operations":draft.operations,
        "operationCapabilities":design_draft_operations::capability_view(&draft.operations, &draft.target_platforms),
        "sourceBinding":draft.source_binding,"targetPlatforms":draft.target_platforms,
        "revision":draft.revision,"status":draft.status,
        "writebackReceiptId":draft.writeback_receipt_id,"historyDepth":draft.history.len(),
        "createdAt":draft.created_at,"updatedAt":draft.updated_at,
    })
}

fn push_history(draft: &mut DesignDraft) {
    draft.history.push(DraftSnapshot {
        revision: draft.revision,
        patches: draft.patches.clone(),
        operations: draft.operations.clone(),
        source_binding: draft.source_binding.clone(),
        target_platforms: draft.target_platforms.clone(),
        status: draft.status.clone(),
        captured_at: chrono::Utc::now().to_rfc3339(),
    });
    if draft.history.len() > MAX_HISTORY {
        draft.history.remove(0);
    }
}

fn normalize_patches(values: Vec<DesignStylePatch>) -> Result<Vec<DesignStylePatch>> {
    if values.len() > 64 {
        bail!("样式 patch 超过 64 项");
    }
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .map(|mut patch| {
            patch.property = clean_property(&patch.property)?;
            if !seen.insert(patch.property.clone()) {
                bail!("样式属性重复: {}", patch.property);
            }
            patch.before = patch.before.map(|value| clean_value(&value)).transpose()?;
            patch.after = clean_value(&patch.after)?;
            patch.unit = patch
                .unit
                .map(|value| clean_text(&value, 16, "unit"))
                .transpose()?;
            Ok(patch)
        })
        .collect()
}

fn normalize_binding(mut binding: DesignSourceBinding) -> Result<DesignSourceBinding> {
    binding.status = binding.status.trim().to_ascii_uppercase();
    if !matches!(binding.status.as_str(), "BOUND" | "CANDIDATE" | "NEEDS_AI") {
        bail!("sourceBinding.status 无效");
    }
    binding.source_file = normalize_relative_path(&binding.source_file)?;
    binding.kind = clean_text(&binding.kind, 64, "sourceBinding.kind")?;
    binding.symbol = binding
        .symbol
        .map(|value| clean_text(&value, 240, "sourceBinding.symbol"))
        .transpose()?;
    binding.confidence = binding.confidence.trim().to_ascii_lowercase();
    if !matches!(binding.confidence.as_str(), "high" | "medium" | "low") {
        bail!("sourceBinding.confidence 无效");
    }
    binding.reason = clean_text(&binding.reason, 500, "sourceBinding.reason")?;
    if let Some(range) = &binding.range {
        if range.end <= range.start || range.end - range.start > 2_000_000 {
            bail!("sourceBinding.range 无效");
        }
    }
    if let Some(revision) = &binding.source_revision {
        if revision.len() > 160 || revision.trim().is_empty() {
            bail!("sourceBinding.sourceRevision 无效");
        }
    }
    Ok(binding)
}

fn validate_binding_target(root: &Path, binding: &DesignSourceBinding) -> Result<()> {
    let path = root
        .join(&binding.source_file)
        .canonicalize()
        .context("DESIGN_SOURCE_FILE_NOT_FOUND：绑定源码不存在")?;
    if !path.starts_with(root) || !path.is_file() {
        bail!("DESIGN_SOURCE_FILE_INVALID：绑定源码越出项目或不是普通文件");
    }
    let size = fs::metadata(path)?.len();
    if let Some(range) = &binding.range {
        if range.end > size {
            bail!("DESIGN_SOURCE_RANGE_STALE：绑定范围超过当前文件大小");
        }
    }
    Ok(())
}

fn normalize_platforms(values: Vec<String>) -> Result<Vec<String>> {
    let platforms = values
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    if platforms.is_empty()
        || platforms.len() > 4
        || platforms
            .iter()
            .any(|value| !matches!(value.as_str(), "web" | "pwa" | "tauri" | "android"))
    {
        bail!("targetPlatforms 只允许 web、pwa、tauri、android 中的一到四项");
    }
    Ok(platforms.into_iter().collect())
}

fn normalize_scope(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if !matches!(
        value.as_str(),
        "instance" | "component" | "route" | "project"
    ) {
        bail!("scope 无效");
    }
    Ok(value)
}

fn clean_property(value: &str) -> Result<String> {
    let value = clean_text(value, 64, "property")?;
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        bail!("样式 property 包含不安全字符");
    }
    Ok(value)
}

fn clean_value(value: &str) -> Result<String> {
    clean_text(value, 500, "style value")
}

fn clean_text(value: &str, max: usize, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.contains('\0') {
        bail!("{field} 为空、过长或包含 NUL");
    }
    Ok(value.to_string())
}

fn normalize_relative_path(value: &str) -> Result<String> {
    let value = value.trim().replace('\\', "/");
    let path = Path::new(&value);
    if value.is_empty()
        || value.len() > 1_000
        || path.is_absolute()
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("sourceFile 不是安全相对路径");
    }
    Ok(value)
}

fn expect_revision(draft: &DesignDraft, expected: u64) -> Result<()> {
    if draft.revision != expected {
        bail!(
            "DESIGN_DRAFT_REVISION_CONFLICT：expected={expected} actual={}",
            draft.revision
        );
    }
    Ok(())
}

fn required_draft_id(arguments: &Value) -> Result<&str> {
    let id = arguments
        .get("draftId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 draftId"))?;
    validate_draft_id(id)?;
    Ok(id)
}

fn required_text<'a>(arguments: &'a Value, key: &str) -> Result<&'a str> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("缺少 {key}"))
}

fn required_revision(arguments: &Value) -> Result<u64> {
    arguments
        .get("expectedRevision")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| anyhow!("缺少 expectedRevision"))
}

fn validate_draft_id(id: &str) -> Result<()> {
    if id.len() != 38
        || !id.starts_with("draft_")
        || !id[6..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("draftId 无效");
    }
    Ok(())
}

fn persist(root: &Path, draft: &DesignDraft) -> Result<()> {
    let directory = draft_directory(root, true)?.context("无法创建设计草稿目录")?;
    fs::write(
        directory.join(format!("{}.json", draft.draft_id)),
        serde_json::to_vec_pretty(draft)?,
    )?;
    Ok(())
}

fn read(root: &Path, draft_id: &str) -> Result<DesignDraft> {
    validate_draft_id(draft_id)?;
    let directory = draft_directory(root, false)?.context("设计草稿目录不存在")?;
    read_draft_file(&directory.join(format!("{draft_id}.json")))
}

fn read_draft_file(path: &Path) -> Result<DesignDraft> {
    let metadata = fs::metadata(path).context("设计草稿不存在")?;
    if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
        bail!("设计草稿无效或过大");
    }
    serde_json::from_slice(&fs::read(path)?).context("设计草稿 JSON 无效")
}

fn draft_directory(root: &Path, create: bool) -> Result<Option<PathBuf>> {
    let directory = root.join(".elon/ui-tuner/headless-design/drafts");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.is_dir() {
        return Ok(None);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("设计草稿目录越出项目");
    }
    Ok(Some(canonical))
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("设计草稿需要绑定项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}

fn default_scope() -> String {
    "instance".into()
}

fn style_patch_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["property","after"],"properties":{"property":{"type":"string","minLength":1,"maxLength":64},"before":{"type":"string","maxLength":500},"after":{"type":"string","minLength":1,"maxLength":500},"unit":{"type":"string","maxLength":16}}})
}

fn source_binding_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["status","sourceFile","kind","confidence","reason"],"properties":{"status":{"enum":["BOUND","CANDIDATE","NEEDS_AI"]},"sourceFile":{"type":"string","minLength":1,"maxLength":1000},"symbol":{"type":"string","maxLength":240},"kind":{"type":"string","minLength":1,"maxLength":64},"range":{"type":"object","additionalProperties":false,"required":["start","end"],"properties":{"start":{"type":"integer","minimum":0},"end":{"type":"integer","minimum":1}}},"sourceRevision":{"type":"string","maxLength":160},"confidence":{"enum":["high","medium","low"]},"reason":{"type":"string","minLength":1,"maxLength":500}}})
}

fn draft_create_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["designSessionId","selector"],"properties":{"designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},"selector":{"type":"string","minLength":1,"maxLength":1000},"scope":{"enum":["instance","component","route","project"],"default":"instance"},"patches":{"type":"array","maxItems":64,"items":style_patch_schema()},"operations":design_draft_operations::operations_schema(),"sourceBinding":source_binding_schema(),"targetPlatforms":{"type":"array","minItems":1,"maxItems":4,"uniqueItems":true,"items":{"enum":["web","pwa","tauri","android"]}}}})
}

fn draft_update_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["draftId","expectedRevision"],"properties":{"draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},"expectedRevision":{"type":"integer","minimum":1},"patches":{"type":"array","maxItems":64,"items":style_patch_schema()},"operations":design_draft_operations::operations_schema(),"sourceBinding":source_binding_schema(),"targetPlatforms":{"type":"array","minItems":1,"maxItems":4,"uniqueItems":true,"items":{"enum":["web","pwa","tauri","android"]}}}})
}

fn draft_id_schema(read_only: bool) -> Value {
    let mut properties = json!({"draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"}});
    if !read_only {
        properties["expectedRevision"] = json!({"type":"integer","minimum":1});
    }
    json!({"type":"object","additionalProperties":false,"required":if read_only {vec!["draftId"]} else {vec!["draftId","expectedRevision"]},"properties":properties})
}

fn begin_writeback_schema() -> Value {
    json!({"type":"object","additionalProperties":false,
    "required":["draftId","expectedRevision","writebackPlanId"],"properties":{
        "draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},
        "expectedRevision":{"type":"integer","minimum":1},
        "writebackPlanId":{"type":"string","pattern":"^writeplan_[a-f0-9]{32}$"}
    }})
}

fn platform_update_schema() -> Value {
    json!({
        "type":"object","additionalProperties":false,"required":["status","method","changedFiles"],
        "properties":{
            "status":{"enum":["PREVIEW","AI_WRITING","SAVED","BUILD_VERIFYING","BUILD_VERIFIED","FAILED","EVIDENCE_MISSING"]},
            "method":{"enum":["PENDING","DETERMINISTIC","CODEX","MIXED"]},
            "changedFiles":{"type":"array","maxItems":256,"items":{"type":"string","minLength":1,"maxLength":1000}},
            "sourceRevisions":{"type":"object","maxProperties":64,"additionalProperties":{"type":"string","maxLength":160}},
            "expectedSourceRevisionBefore":{"type":"string","maxLength":160},
            "buildEvidence":{"type":"object","maxProperties":32},
            "aiTaskId":{"type":"string","maxLength":160},
            "error":{"type":"string","maxLength":2000}
        }
    })
}

fn complete_writeback_schema() -> Value {
    let update = platform_update_schema();
    json!({
        "type":"object","additionalProperties":false,
        "required":["draftId","expectedRevision","platformResults"],
        "properties":{
            "draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},
            "expectedRevision":{"type":"integer","minimum":1},
            "platformResults":{
                "type":"object","additionalProperties":false,"minProperties":1,"maxProperties":4,
                "properties":{"web":update.clone(),"pwa":update.clone(),"tauri":update.clone(),"android":update}
            }
        }
    })
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":input_schema,"annotations":{"readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":read_only,"openWorldHint":false}})
}
