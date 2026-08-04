use std::{fs, path::PathBuf};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use sha2::Digest;

use super::{
    broker::LiveUiSession,
    design_regression_store::{
        self as store, DesignRegressionBaseline, DesignRegressionTask, RegressionThresholds,
    },
    design_session_store,
};

const CREATE_BASELINE_TOOL: &str = "ui_create_design_regression_baseline";
const GET_BASELINE_TOOL: &str = "ui_get_design_regression_baseline";
const PLAN_COMPARISON_TOOL: &str = "ui_plan_design_regression_comparison";
const GET_COMPARISON_TOOL: &str = "ui_get_design_regression_comparison";
const COMPLETE_COMPARISON_TOOL: &str = "ui_complete_design_regression_comparison";

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            CREATE_BASELINE_TOOL,
            "把 designSession 已存在且通过哈希校验的 PNG/UI tree 固化为修改前回归基线；不启动捕获。",
            baseline_create_schema(),
            false,
        ),
        tool(
            GET_BASELINE_TOOL,
            "读取修改前视觉/语义基线的证据引用和平台状态；不嵌入图片或 UI tree 正文。",
            id_schema("baselineId", "^baseline_[a-f0-9]{32}$", false),
            true,
        ),
        tool(
            PLAN_COMPARISON_TOOL,
            "将基线与修改后 designSession 的已验证证据编译成可领取的视觉/语义比较任务；不执行比较。",
            comparison_plan_schema(),
            false,
        ),
        tool(
            GET_COMPARISON_TOOL,
            "读取回归比较任务、阈值、前后证据哈希与结果状态。",
            id_schema("comparisonId", "^compare_[a-f0-9]{32}$", false),
            true,
        ),
        tool(
            COMPLETE_COMPARISON_TOOL,
            "提交比较器产出的视觉/语义指标和已落盘证据；节点按固定阈值计算 PASSED/FAILED。",
            comparison_complete_schema(),
            false,
        ),
    ]
}

pub(super) fn is_tool(name: &str) -> bool {
    matches!(
        name,
        CREATE_BASELINE_TOOL
            | GET_BASELINE_TOOL
            | PLAN_COMPARISON_TOOL
            | GET_COMPARISON_TOOL
            | COMPLETE_COMPARISON_TOOL
    )
}

pub(super) fn call(session: &LiveUiSession, name: &str, arguments: Value) -> Result<Value> {
    let root = canonical_root(session)?;
    match name {
        CREATE_BASELINE_TOOL => create_baseline(session, &root, &arguments),
        GET_BASELINE_TOOL => get_baseline(&root, required_text(&arguments, "baselineId")?),
        PLAN_COMPARISON_TOOL => plan_comparison(&root, &arguments),
        GET_COMPARISON_TOOL => get_comparison(&root, required_text(&arguments, "comparisonId")?),
        COMPLETE_COMPARISON_TOOL => complete_comparison(&root, &arguments),
        _ => bail!("未知设计回归契约工具: {name}"),
    }
}

fn create_baseline(
    session: &LiveUiSession,
    root: &std::path::Path,
    arguments: &Value,
) -> Result<Value> {
    let design_session_id = required_text(arguments, "designSessionId")?;
    design_session_store::validate_design_session_id(design_session_id)?;
    let record = design_session_store::read_record(root, design_session_id)?;
    let evidence = verified_evidence(root, &record)?;
    let draft_id = optional_text(arguments, "draftId");
    if let Some(draft_id) = draft_id {
        validate_draft_session(session, draft_id, design_session_id)?;
    }
    let pixels = store::evidence_ref(evidence.get("artifact").context("证据缺少 artifact")?)?;
    let ui_tree = store::evidence_ref(evidence.get("uiTree").context("证据缺少 uiTree")?)?;
    let baseline_id =
        store::baseline_id(design_session_id, &pixels.sha256, &ui_tree.sha256, draft_id);
    if let Ok(existing) = store::read_baseline(root, &baseline_id) {
        return Ok(baseline_response("UNCHANGED", &existing));
    }
    let label = optional_text(arguments, "label").map(|value| clean(value, 120));
    let native_host = evidence
        .pointer("/nativeHost/artifact")
        .map(store::evidence_ref)
        .transpose()?;
    let baseline = DesignRegressionBaseline {
        schema_version: 1,
        baseline_id,
        revision: 1,
        design_session_id: design_session_id.to_string(),
        draft_id: draft_id.map(str::to_string),
        platform: record.platform.as_str().to_string(),
        route: record.route,
        state: record.state,
        viewport: record.viewport,
        pixels,
        ui_tree,
        native_host,
        label,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store::persist_baseline(root, &baseline)?;
    Ok(baseline_response("CREATED", &baseline))
}

fn get_baseline(root: &std::path::Path, baseline_id: &str) -> Result<Value> {
    let baseline = store::read_baseline(root, baseline_id)?;
    Ok(baseline_response("READ", &baseline))
}

fn plan_comparison(root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let baseline_id = required_text(arguments, "baselineId")?;
    let baseline = store::read_baseline(root, baseline_id)?;
    let after_session_id = required_text(arguments, "afterDesignSessionId")?;
    design_session_store::validate_design_session_id(after_session_id)?;
    let after = design_session_store::read_record(root, after_session_id)?;
    if baseline.platform != after.platform.as_str() || baseline.route != after.route {
        bail!("DESIGN_REGRESSION_TARGET_MISMATCH：前后平台或 route 不一致");
    }
    let thresholds = thresholds(arguments.get("thresholds"))?;
    if thresholds.require_same_viewport && baseline.viewport != after.viewport {
        bail!("DESIGN_REGRESSION_VIEWPORT_MISMATCH：前后 viewport 不一致");
    }
    let evidence = verified_evidence(root, &after)?;
    let after_pixels = store::evidence_ref(evidence.get("artifact").context("证据缺少 artifact")?)?;
    let after_ui_tree = store::evidence_ref(evidence.get("uiTree").context("证据缺少 uiTree")?)?;
    let changed_selectors = selector_list(arguments.get("changedSelectors"), 64)?;
    let comparison_id = store::comparison_id(
        baseline_id,
        &after_pixels.sha256,
        &after_ui_tree.sha256,
        &thresholds,
        &changed_selectors,
    )?;
    if let Ok(existing) = store::read_task(root, &comparison_id) {
        return Ok(task_response("UNCHANGED", &existing));
    }
    let now = chrono::Utc::now().to_rfc3339();
    let task = DesignRegressionTask {
        schema_version: 1,
        comparison_id,
        revision: 1,
        baseline_id: baseline_id.to_string(),
        before_design_session_id: baseline.design_session_id,
        after_design_session_id: after_session_id.to_string(),
        platform: baseline.platform,
        route: baseline.route,
        before_pixels: baseline.pixels,
        after_pixels,
        before_ui_tree: baseline.ui_tree,
        after_ui_tree,
        thresholds,
        changed_selectors,
        status: "READY_TO_COMPARE".to_string(),
        result: None,
        created_at: now.clone(),
        updated_at: now,
    };
    store::persist_task(root, &task)?;
    Ok(task_response("PLANNED", &task))
}

fn get_comparison(root: &std::path::Path, comparison_id: &str) -> Result<Value> {
    let task = store::read_task(root, comparison_id)?;
    Ok(task_response("READ", &task))
}

fn complete_comparison(root: &std::path::Path, arguments: &Value) -> Result<Value> {
    let comparison_id = required_text(arguments, "comparisonId")?;
    let expected = required_u64(arguments, "expectedRevision")?;
    let mut task = store::read_task(root, comparison_id)?;
    if task.revision != expected {
        bail!(
            "DESIGN_REGRESSION_REVISION_CONFLICT：expected={expected} actual={}",
            task.revision
        );
    }
    if task.status != "READY_TO_COMPARE" {
        bail!("DESIGN_REGRESSION_STATE_CONFLICT：比较任务已经结算");
    }
    let pixel_diff_ratio = arguments
        .get("pixelDiffRatio")
        .and_then(Value::as_f64)
        .filter(|value| (0.0..=1.0).contains(value))
        .context("pixelDiffRatio 必须在 0..1")?;
    let missing = selector_list(arguments.get("missingSelectors"), 256)?;
    let changed = selector_list(arguments.get("changedSelectors"), 256)?;
    let added = selector_list(arguments.get("addedSelectors"), 256)?;
    let visual_artifact = verified_result_artifact(
        root,
        arguments
            .get("visualDiffArtifact")
            .context("缺少 visualDiffArtifact")?,
    )?;
    let semantic_artifact = verified_result_artifact(
        root,
        arguments
            .get("semanticDiffArtifact")
            .context("缺少 semanticDiffArtifact")?,
    )?;
    let ignored_missing = missing
        .iter()
        .filter(|selector| !task.thresholds.ignore_selectors.contains(selector))
        .count() as u64;
    let unexpected_changed = changed
        .iter()
        .filter(|selector| {
            !task.changed_selectors.contains(selector)
                && !task.thresholds.ignore_selectors.contains(selector)
        })
        .count() as u64;
    let passed = pixel_diff_ratio <= task.thresholds.max_pixel_diff_ratio
        && ignored_missing <= task.thresholds.max_missing_selectors
        && unexpected_changed <= task.thresholds.max_changed_selectors;
    task.revision += 1;
    task.status = if passed { "PASSED" } else { "FAILED" }.to_string();
    task.updated_at = chrono::Utc::now().to_rfc3339();
    task.result = Some(json!({
        "comparatorId":clean(required_text(arguments,"comparatorId")?,120),
        "pixelDiffRatio":pixel_diff_ratio,
        "changedPixelCount":arguments.get("changedPixelCount").and_then(Value::as_u64),
        "missingSelectors":missing,"changedSelectors":changed,"addedSelectors":added,
        "unexpectedMissingCount":ignored_missing,"unexpectedChangedCount":unexpected_changed,
        "visualDiffArtifact":visual_artifact,"semanticDiffArtifact":semantic_artifact,
        "artifactsVerified":true,"passed":passed
    }));
    store::persist_task(root, &task)?;
    Ok(task_response("COMPLETED", &task))
}

fn verified_evidence<'a>(
    root: &std::path::Path,
    record: &'a design_session_store::DesignSessionRecord,
) -> Result<&'a Value> {
    let evidence = record
        .last_evidence
        .as_ref()
        .context("designSession 尚无可固化的完整证据")?;
    design_session_store::read_verified_pixels(root, record)?;
    design_session_store::read_verified_tree(root, evidence)?;
    Ok(evidence)
}

fn validate_draft_session(
    session: &LiveUiSession,
    draft_id: &str,
    design_session_id: &str,
) -> Result<()> {
    let result =
        super::design_drafts::call(session, "ui_get_design_draft", json!({"draftId":draft_id}))?;
    if result
        .pointer("/draft/designSessionId")
        .and_then(Value::as_str)
        != Some(design_session_id)
    {
        bail!("DESIGN_REGRESSION_DRAFT_MISMATCH：draft 不属于 designSession");
    }
    Ok(())
}

fn thresholds(value: Option<&Value>) -> Result<RegressionThresholds> {
    let empty = Value::Null;
    let value = value.unwrap_or(&empty);
    let ratio = value
        .get("maxPixelDiffRatio")
        .and_then(Value::as_f64)
        .unwrap_or(0.01);
    if !(0.0..=1.0).contains(&ratio) {
        bail!("maxPixelDiffRatio 必须在 0..1");
    }
    let max_missing_selectors = value
        .get("maxMissingSelectors")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let max_changed_selectors = value
        .get("maxChangedSelectors")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if max_missing_selectors > 10_000 || max_changed_selectors > 10_000 {
        bail!("语义 selector 阈值不能超过 10000");
    }
    Ok(RegressionThresholds {
        max_pixel_diff_ratio: ratio,
        max_missing_selectors,
        max_changed_selectors,
        require_same_viewport: value
            .get("requireSameViewport")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        ignore_selectors: selector_list(value.get("ignoreSelectors"), 64)?,
    })
}

fn verified_result_artifact(root: &std::path::Path, value: &Value) -> Result<Value> {
    let reference = store::evidence_ref(value)?;
    let relative = std::path::Path::new(&reference.path);
    if relative.is_absolute() || relative.components().any(|part| part.as_os_str() == "..") {
        bail!("比较结果 artifact path 无效");
    }
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root) || !path.is_file() || fs::metadata(&path)?.len() > 64 * 1024 * 1024 {
        bail!("比较结果 artifact 越出项目、不是文件或过大");
    }
    let actual = hex::encode(sha2::Sha256::digest(fs::read(path)?));
    if reference.sha256 != actual && reference.sha256 != format!("sha256:{actual}") {
        bail!("比较结果 artifact SHA-256 不匹配");
    }
    Ok(json!({"path":reference.path,"sha256":reference.sha256}))
}

fn selector_list(value: Option<&Value>, max: usize) -> Result<Vec<String>> {
    let values = value.and_then(Value::as_array).cloned().unwrap_or_default();
    if values.len() > max {
        bail!("selector 列表超过 {max} 项");
    }
    values
        .into_iter()
        .map(|value| {
            let value = value.as_str().context("selector 必须是字符串")?.trim();
            if value.is_empty() || value.len() > 1_000 || value.contains('\0') {
                bail!("selector 为空、过长或包含 NUL");
            }
            Ok(value.to_string())
        })
        .collect()
}

fn baseline_response(action: &str, baseline: &DesignRegressionBaseline) -> Value {
    json!({"schema":"elon.ui-design-regression-baseline.v1","action":action,
        "baseline":store::baseline_view(baseline),"runtimeStarted":false,"sourceModified":false})
}

fn task_response(action: &str, task: &DesignRegressionTask) -> Value {
    json!({"schema":"elon.ui-design-regression-comparison.v1","action":action,
        "comparison":store::task_view(task),"runtimeStarted":false,"sourceModified":false})
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("设计回归契约需要项目目录")?,
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
        "readOnlyHint":read_only,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}

fn id_schema(name: &str, pattern: &str, with_revision: bool) -> Value {
    let mut properties = json!({});
    properties[name] = json!({"type":"string","pattern":pattern});
    let mut required = vec![name];
    if with_revision {
        properties["expectedRevision"] = json!({"type":"integer","minimum":1});
        required.push("expectedRevision");
    }
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}

fn baseline_create_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["designSessionId"],"properties":{
        "designSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
        "draftId":{"type":"string","pattern":"^draft_[a-f0-9]{32}$"},
        "label":{"type":"string","maxLength":120}
    }})
}

fn thresholds_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{
        "maxPixelDiffRatio":{"type":"number","minimum":0,"maximum":1,"default":0.01},
        "maxMissingSelectors":{"type":"integer","minimum":0,"maximum":10000,"default":0},
        "maxChangedSelectors":{"type":"integer","minimum":0,"maximum":10000,"default":0},
        "requireSameViewport":{"type":"boolean","default":true},
        "ignoreSelectors":{"type":"array","maxItems":64,"items":{"type":"string","minLength":1,"maxLength":1000}}
    }})
}

fn comparison_plan_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["baselineId","afterDesignSessionId"],"properties":{
        "baselineId":{"type":"string","pattern":"^baseline_[a-f0-9]{32}$"},
        "afterDesignSessionId":{"type":"string","pattern":"^design_[a-f0-9]{32}$"},
        "changedSelectors":{"type":"array","maxItems":64,"items":{"type":"string","minLength":1,"maxLength":1000}},
        "thresholds":thresholds_schema()
    }})
}

fn artifact_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["path","sha256"],"properties":{
        "path":{"type":"string","minLength":1,"maxLength":2048},
        "sha256":{"type":"string","minLength":64,"maxLength":71}
    }})
}

fn comparison_complete_schema() -> Value {
    let mut properties =
        id_schema("comparisonId", "^compare_[a-f0-9]{32}$", true)["properties"].clone();
    properties["comparatorId"] = json!({"type":"string","minLength":1,"maxLength":120});
    properties["pixelDiffRatio"] = json!({"type":"number","minimum":0,"maximum":1});
    properties["changedPixelCount"] = json!({"type":"integer","minimum":0});
    for name in ["missingSelectors", "changedSelectors", "addedSelectors"] {
        properties[name] = json!({"type":"array","maxItems":256,"items":{"type":"string","minLength":1,"maxLength":1000}});
    }
    properties["visualDiffArtifact"] = artifact_schema();
    properties["semanticDiffArtifact"] = artifact_schema();
    json!({"type":"object","additionalProperties":false,
        "required":["comparisonId","expectedRevision","comparatorId","pixelDiffRatio","missingSelectors","changedSelectors","addedSelectors","visualDiffArtifact","semanticDiffArtifact"],
        "properties":properties})
}
