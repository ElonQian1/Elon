use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAX_RECORD_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RegressionEvidenceRef {
    pub(super) path: String,
    pub(super) sha256: String,
    pub(super) width: Option<u64>,
    pub(super) height: Option<u64>,
    pub(super) node_count: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesignRegressionBaseline {
    pub(super) schema_version: u32,
    pub(super) baseline_id: String,
    pub(super) revision: u64,
    pub(super) design_session_id: String,
    pub(super) draft_id: Option<String>,
    pub(super) platform: String,
    pub(super) route: String,
    pub(super) state: String,
    pub(super) viewport: Value,
    pub(super) pixels: RegressionEvidenceRef,
    pub(super) ui_tree: RegressionEvidenceRef,
    pub(super) native_host: Option<RegressionEvidenceRef>,
    pub(super) label: Option<String>,
    pub(super) created_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RegressionThresholds {
    pub(super) max_pixel_diff_ratio: f64,
    pub(super) max_missing_selectors: u64,
    pub(super) max_changed_selectors: u64,
    pub(super) require_same_viewport: bool,
    pub(super) ignore_selectors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesignRegressionTask {
    pub(super) schema_version: u32,
    pub(super) comparison_id: String,
    pub(super) revision: u64,
    pub(super) baseline_id: String,
    pub(super) before_design_session_id: String,
    pub(super) after_design_session_id: String,
    pub(super) platform: String,
    pub(super) route: String,
    pub(super) before_pixels: RegressionEvidenceRef,
    pub(super) after_pixels: RegressionEvidenceRef,
    pub(super) before_ui_tree: RegressionEvidenceRef,
    pub(super) after_ui_tree: RegressionEvidenceRef,
    pub(super) thresholds: RegressionThresholds,
    pub(super) changed_selectors: Vec<String>,
    pub(super) status: String,
    pub(super) result: Option<Value>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
}

pub(super) fn evidence_ref(value: &Value) -> Result<RegressionEvidenceRef> {
    let path = required_text(value, "path")?;
    let sha256 = required_text(value, "sha256")?;
    if path.len() > 2_048 || sha256.len() > 160 || path.contains('\0') || sha256.contains('\0') {
        bail!("回归证据引用无效");
    }
    Ok(RegressionEvidenceRef {
        path: path.to_string(),
        sha256: sha256.to_string(),
        width: value.get("width").and_then(Value::as_u64),
        height: value.get("height").and_then(Value::as_u64),
        node_count: value.get("nodeCount").and_then(Value::as_u64),
    })
}

pub(super) fn baseline_id(
    design_session_id: &str,
    pixels_sha: &str,
    tree_sha: &str,
    draft_id: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(design_session_id.as_bytes());
    hasher.update(pixels_sha.as_bytes());
    hasher.update(tree_sha.as_bytes());
    hasher.update(draft_id.unwrap_or("").as_bytes());
    format!("baseline_{}", &hex::encode(hasher.finalize())[..32])
}

pub(super) fn comparison_id(
    baseline_id: &str,
    after_pixels_sha: &str,
    after_tree_sha: &str,
    thresholds: &RegressionThresholds,
    changed_selectors: &[String],
) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(baseline_id.as_bytes());
    hasher.update(after_pixels_sha.as_bytes());
    hasher.update(after_tree_sha.as_bytes());
    hasher.update(serde_json::to_vec(thresholds)?);
    hasher.update(serde_json::to_vec(changed_selectors)?);
    Ok(format!("compare_{}", &hex::encode(hasher.finalize())[..32]))
}

pub(super) fn persist_baseline(root: &Path, baseline: &DesignRegressionBaseline) -> Result<()> {
    persist(
        &record_directory(root, true)?.join(format!("{}.json", baseline.baseline_id)),
        baseline,
    )
}

pub(super) fn read_baseline(root: &Path, id: &str) -> Result<DesignRegressionBaseline> {
    validate_id(id, "baseline_")?;
    read(
        &record_directory(root, false)?.join(format!("{id}.json")),
        "回归基线",
    )
}

pub(super) fn persist_task(root: &Path, task: &DesignRegressionTask) -> Result<()> {
    persist(
        &record_directory(root, true)?.join(format!("{}.json", task.comparison_id)),
        task,
    )
}

pub(super) fn read_task(root: &Path, id: &str) -> Result<DesignRegressionTask> {
    validate_id(id, "compare_")?;
    read(
        &record_directory(root, false)?.join(format!("{id}.json")),
        "回归比较任务",
    )
}

pub(super) fn baseline_view(baseline: &DesignRegressionBaseline) -> Value {
    json!({
        "schemaVersion":baseline.schema_version,"baselineId":baseline.baseline_id,
        "revision":baseline.revision,"designSessionId":baseline.design_session_id,
        "draftId":baseline.draft_id,"platform":baseline.platform,"route":baseline.route,
        "state":baseline.state,"viewport":baseline.viewport,"pixels":baseline.pixels,
        "uiTree":baseline.ui_tree,"nativeHost":baseline.native_host,"label":baseline.label,
        "createdAt":baseline.created_at,"contentEmbedded":false
    })
}

pub(super) fn task_view(task: &DesignRegressionTask) -> Value {
    json!({
        "schemaVersion":task.schema_version,"comparisonId":task.comparison_id,
        "revision":task.revision,"baselineId":task.baseline_id,
        "beforeDesignSessionId":task.before_design_session_id,
        "afterDesignSessionId":task.after_design_session_id,"platform":task.platform,
        "route":task.route,"beforePixels":task.before_pixels,"afterPixels":task.after_pixels,
        "beforeUiTree":task.before_ui_tree,"afterUiTree":task.after_ui_tree,
        "thresholds":task.thresholds,"changedSelectors":task.changed_selectors,
        "status":task.status,"result":task.result,"createdAt":task.created_at,
        "updatedAt":task.updated_at,"contentEmbedded":false
    })
}

fn persist(path: &Path, value: &impl Serialize) -> Result<()> {
    crate::node_agent_atomic_file::write(path, &serde_json::to_vec_pretty(value)?)
}

fn read<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T> {
    let metadata = fs::metadata(path).with_context(|| format!("{label}不存在"))?;
    if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
        bail!("{label}无效或过大");
    }
    serde_json::from_slice(&fs::read(path)?).with_context(|| format!("{label} JSON 无效"))
}

fn record_directory(root: &Path, create: bool) -> Result<PathBuf> {
    let directory = root.join(".elon/ui-tuner/headless-design/regressions");
    if create {
        fs::create_dir_all(&directory)?;
    }
    if !directory.exists() {
        return Ok(directory);
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("设计回归记录目录越出项目");
    }
    Ok(canonical)
}

fn validate_id(value: &str, prefix: &str) -> Result<()> {
    if value.len() != prefix.len() + 32
        || !value.starts_with(prefix)
        || !value[prefix.len()..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("设计回归记录 ID 无效");
    }
    Ok(())
}

fn required_text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("证据缺少 {key}"))
}
