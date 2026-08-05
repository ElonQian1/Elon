use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use image::RgbaImage;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{
    broker::LiveUiSession,
    design_regression_store::{self as store, DesignRegressionTask, RegressionEvidenceRef},
};

const RUN_TOOL: &str = "ui_run_design_regression_comparison";
const COMPLETE_TOOL: &str = "ui_complete_design_regression_comparison";
const COMPARATOR_ID: &str = "elon-node-local-regression-v1";
const MAX_PIXEL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TREE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PixelMetrics {
    before_width: u32,
    before_height: u32,
    after_width: u32,
    after_height: u32,
    compared_pixel_count: u64,
    changed_pixel_count: u64,
    pixel_diff_ratio: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SemanticMetrics {
    before_selector_count: usize,
    after_selector_count: usize,
    missing_selectors: Vec<String>,
    changed_selectors: Vec<String>,
    added_selectors: Vec<String>,
}

pub(super) fn tool_definitions() -> Vec<Value> {
    vec![json!({
        "name":RUN_TOOL,
        "description":"在节点本机验证比较任务固定的 PNG/UI tree 哈希，运行确定性像素与 selector 比较，生成项目内紧凑 diff artifact 并按任务阈值结算；不启动页面、不嵌入图片。",
        "inputSchema":{
            "type":"object","additionalProperties":false,
            "required":["comparisonId","expectedRevision"],
            "properties":{
                "comparisonId":{"type":"string","pattern":"^compare_[a-f0-9]{32}$"},
                "expectedRevision":{"type":"integer","minimum":1}
            }
        },
        "annotations":{"readOnlyHint":false,"destructiveHint":false,
            "idempotentHint":false,"openWorldHint":false}
    })]
}

pub(super) fn is_tool(name: &str) -> bool {
    name == RUN_TOOL
}

pub(super) fn call(session: &LiveUiSession, arguments: Value) -> Result<Value> {
    let root = canonical_root(session)?;
    let comparison_id = required_text(&arguments, "comparisonId")?;
    let expected_revision = arguments
        .get("expectedRevision")
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .context("缺少 expectedRevision")?;
    let task = store::read_task(&root, comparison_id)?;
    if task.revision != expected_revision {
        bail!(
            "DESIGN_REGRESSION_REVISION_CONFLICT：expected={expected_revision} actual={}",
            task.revision
        );
    }
    if task.status != "READY_TO_COMPARE" {
        bail!("DESIGN_REGRESSION_STATE_CONFLICT：只有 READY_TO_COMPARE 任务可以运行本机比较器");
    }
    run(session, &root, &task)
}

fn run(session: &LiveUiSession, root: &Path, task: &DesignRegressionTask) -> Result<Value> {
    let before_pixels = read_verified_artifact(root, &task.before_pixels, MAX_PIXEL_BYTES)?;
    let after_pixels = read_verified_artifact(root, &task.after_pixels, MAX_PIXEL_BYTES)?;
    let before_tree = read_verified_artifact(root, &task.before_ui_tree, MAX_TREE_BYTES)?;
    let after_tree = read_verified_artifact(root, &task.after_ui_tree, MAX_TREE_BYTES)?;

    let pixel_metrics = compare_pixels(&before_pixels, &after_pixels)?;
    let semantic_metrics = compare_semantics(&before_tree, &after_tree)?;
    let artifacts = write_artifacts(root, task, &pixel_metrics, &semantic_metrics)?;

    let result = super::design_regression_contract::call(
        session,
        COMPLETE_TOOL,
        json!({
            "comparisonId":task.comparison_id,
            "expectedRevision":task.revision,
            "comparatorId":COMPARATOR_ID,
            "pixelDiffRatio":pixel_metrics.pixel_diff_ratio,
            "changedPixelCount":pixel_metrics.changed_pixel_count,
            "missingSelectors":semantic_metrics.missing_selectors,
            "changedSelectors":semantic_metrics.changed_selectors,
            "addedSelectors":semantic_metrics.added_selectors,
            "visualDiffArtifact":artifacts.0,
            "semanticDiffArtifact":artifacts.1,
        }),
    )?;
    Ok(result)
}

fn read_verified_artifact(
    root: &Path,
    reference: &RegressionEvidenceRef,
    max: u64,
) -> Result<Vec<u8>> {
    let supplied = PathBuf::from(&reference.path);
    let path = if supplied.is_absolute() {
        supplied
    } else {
        root.join(supplied)
    }
    .canonicalize()
    .context("回归比较输入 artifact 不存在")?;
    let metadata = fs::metadata(&path)?;
    if !path.starts_with(root) || !metadata.is_file() || metadata.len() == 0 || metadata.len() > max
    {
        bail!("回归比较输入 artifact 越出项目、不是普通文件或超过大小上限");
    }
    let bytes = fs::read(path)?;
    let actual = hex::encode(Sha256::digest(&bytes));
    if !reference.sha256.eq_ignore_ascii_case(&actual)
        && !reference
            .sha256
            .eq_ignore_ascii_case(&format!("sha256:{actual}"))
    {
        bail!("DESIGN_REGRESSION_INPUT_DRIFT：比较输入 artifact SHA-256 不匹配");
    }
    Ok(bytes)
}

fn compare_pixels(before: &[u8], after: &[u8]) -> Result<PixelMetrics> {
    let before = image::load_from_memory(before)
        .context("修改前像素工件无法解码")?
        .to_rgba8();
    let after = image::load_from_memory(after)
        .context("修改后像素工件无法解码")?
        .to_rgba8();
    let width = before.width().max(after.width());
    let height = before.height().max(after.height());
    let compared = u64::from(width) * u64::from(height);
    let mut changed = 0u64;
    for y in 0..height {
        for x in 0..width {
            if pixel_at(&before, x, y) != pixel_at(&after, x, y) {
                changed += 1;
            }
        }
    }
    let ratio = if compared == 0 {
        0.0
    } else {
        changed as f64 / compared as f64
    };
    Ok(PixelMetrics {
        before_width: before.width(),
        before_height: before.height(),
        after_width: after.width(),
        after_height: after.height(),
        compared_pixel_count: compared,
        changed_pixel_count: changed,
        pixel_diff_ratio: ratio,
    })
}

fn pixel_at(image: &RgbaImage, x: u32, y: u32) -> Option<[u8; 4]> {
    (x < image.width() && y < image.height()).then(|| image.get_pixel(x, y).0)
}

fn compare_semantics(before: &[u8], after: &[u8]) -> Result<SemanticMetrics> {
    let before: Value = serde_json::from_slice(before).context("修改前 UI tree JSON 无效")?;
    let after: Value = serde_json::from_slice(after).context("修改后 UI tree JSON 无效")?;
    let before = semantic_nodes(&before)?;
    let after = semantic_nodes(&after)?;
    let missing_selectors = before
        .keys()
        .filter(|selector| !after.contains_key(*selector))
        .cloned()
        .collect();
    let added_selectors = after
        .keys()
        .filter(|selector| !before.contains_key(*selector))
        .cloned()
        .collect();
    let changed_selectors = before
        .iter()
        .filter_map(|(selector, signature)| {
            after
                .get(selector)
                .filter(|after_signature| *after_signature != signature)
                .map(|_| selector.clone())
        })
        .collect();
    Ok(SemanticMetrics {
        before_selector_count: before.len(),
        after_selector_count: after.len(),
        missing_selectors,
        changed_selectors,
        added_selectors,
    })
}

fn semantic_nodes(tree: &Value) -> Result<BTreeMap<String, Value>> {
    let nodes = tree
        .get("nodes")
        .and_then(Value::as_array)
        .context("UI tree 缺少 nodes")?;
    let parents = nodes
        .iter()
        .filter_map(|node| node.get("parentSelector").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let mut result = BTreeMap::new();
    for node in nodes {
        let selector = node
            .get("selector")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .context("UI tree node 缺少 selector")?;
        if result.contains_key(selector) {
            bail!("UI tree selector 重复，不能确定性比较: {selector}");
        }
        let label = (!parents.contains(selector))
            .then(|| node.get("label").cloned().unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        result.insert(
            selector.to_string(),
            json!({
                "tag":node.get("tag"),"role":node.get("role"),"label":label,
                "interactive":node.get("interactive"),"disabled":node.get("disabled"),
                "checked":node.get("checked"),"selected":node.get("selected"),
                "inputType":node.get("inputType"),"style":node.get("style")
            }),
        );
    }
    Ok(result)
}

fn write_artifacts(
    root: &Path,
    task: &DesignRegressionTask,
    pixels: &PixelMetrics,
    semantics: &SemanticMetrics,
) -> Result<(Value, Value)> {
    let directory = root.join(".elon/ui-tuner/headless-design/regressions");
    fs::create_dir_all(&directory)?;
    let directory = directory.canonicalize()?;
    if !directory.starts_with(root) {
        bail!("回归比较输出目录越出项目");
    }
    let visual_relative = format!(
        ".elon/ui-tuner/headless-design/regressions/{}.local-visual.json",
        task.comparison_id
    );
    let semantic_relative = format!(
        ".elon/ui-tuner/headless-design/regressions/{}.local-semantic.json",
        task.comparison_id
    );
    let visual = serde_json::to_vec_pretty(&json!({
        "schema":"elon.ui-design-local-visual-diff.v1","comparatorId":COMPARATOR_ID,
        "comparisonId":task.comparison_id,"beforeSha256":task.before_pixels.sha256,
        "afterSha256":task.after_pixels.sha256,"metrics":pixels,"contentEmbedded":false
    }))?;
    let semantic = serde_json::to_vec_pretty(&json!({
        "schema":"elon.ui-design-local-semantic-diff.v1","comparatorId":COMPARATOR_ID,
        "comparisonId":task.comparison_id,"beforeSha256":task.before_ui_tree.sha256,
        "afterSha256":task.after_ui_tree.sha256,"metrics":semantics,"contentEmbedded":false
    }))?;
    crate::node_agent_atomic_file::write(&root.join(&visual_relative), &visual)?;
    crate::node_agent_atomic_file::write(&root.join(&semantic_relative), &semantic)?;
    Ok((
        json!({"path":visual_relative,"sha256":hex::encode(Sha256::digest(&visual))}),
        json!({"path":semantic_relative,"sha256":hex::encode(Sha256::digest(&semantic))}),
    ))
}

fn canonical_root(session: &LiveUiSession) -> Result<PathBuf> {
    PathBuf::from(
        session
            .project_root
            .as_deref()
            .context("本机设计回归比较器需要项目目录")?,
    )
    .canonicalize()
    .context("项目目录不存在")
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}
