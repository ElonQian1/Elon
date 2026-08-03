use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::design_targets::{DesignPlatform, DesignTarget};

const MAX_TREE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesignSessionRecord {
    pub(super) schema_version: u32,
    pub(super) design_session_id: String,
    pub(super) mcp_session_id: String,
    pub(super) platform: DesignPlatform,
    pub(super) target: DesignTarget,
    pub(super) route: String,
    pub(super) url: Option<String>,
    pub(super) viewport: Value,
    pub(super) state: String,
    pub(super) last_evidence: Option<Value>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
}

pub(super) fn persist_record(root: &Path, record: &DesignSessionRecord) -> Result<()> {
    let path = record_path(root, &record.design_session_id, true)?;
    fs::write(path, serde_json::to_vec_pretty(record)?)?;
    Ok(())
}

pub(super) fn read_record(root: &Path, id: &str) -> Result<DesignSessionRecord> {
    let path = record_path(root, id, false)?;
    serde_json::from_slice(&fs::read(path)?).context("后台设计会话 JSON 无效")
}

pub(super) fn validate_design_session_id(value: &str) -> Result<()> {
    if value.len() != 39
        || !value.starts_with("design_")
        || !value[7..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        bail!("designSessionId 无效");
    }
    Ok(())
}

pub(super) fn read_verified_tree(root: &Path, evidence: &Value) -> Result<Value> {
    let path = evidence
        .pointer("/uiTree/path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("后台设计证据缺少 UI tree path"))?;
    let expected = evidence
        .pointer("/uiTree/sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("后台设计证据缺少 UI tree sha256"))?;
    let path = PathBuf::from(path)
        .canonicalize()
        .context("后台设计 UI tree 工件不存在")?;
    if !path.starts_with(root) || fs::metadata(&path)?.len() > MAX_TREE_BYTES {
        bail!("后台设计 UI tree 越出项目或超过大小上限");
    }
    let bytes = fs::read(path)?;
    if !expected.eq_ignore_ascii_case(&hex::encode(Sha256::digest(&bytes))) {
        bail!("后台设计 UI tree 哈希不匹配");
    }
    serde_json::from_slice(&bytes).context("后台设计 UI tree JSON 无效")
}

fn record_path(root: &Path, id: &str, create: bool) -> Result<PathBuf> {
    validate_design_session_id(id)?;
    let directory = root.join(".elon/ui-tuner/headless-design/sessions");
    if create {
        fs::create_dir_all(&directory)?;
    }
    let canonical = directory.canonicalize().context("后台设计会话目录不存在")?;
    if !canonical.starts_with(root) {
        bail!("后台设计会话目录越出项目");
    }
    Ok(canonical.join(format!("{id}.json")))
}
