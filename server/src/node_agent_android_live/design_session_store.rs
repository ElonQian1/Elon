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
const MAX_PIXEL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SESSION_RECORD_BYTES: u64 = 512 * 1024;

pub(super) struct VerifiedPixelArtifact {
    pub(super) bytes: Vec<u8>,
    pub(super) media_type: String,
    pub(super) sha256: String,
}

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

pub(super) fn list_records(root: &Path, limit: usize) -> Result<(Vec<DesignSessionRecord>, usize)> {
    let directory = root.join(".elon/ui-tuner/headless-design/sessions");
    if !directory.is_dir() {
        return Ok((Vec::new(), 0));
    }
    let canonical = directory.canonicalize()?;
    if !canonical.starts_with(root) {
        bail!("后台设计会话目录越出项目");
    }
    let mut records = Vec::new();
    let mut invalid = 0usize;
    for entry in fs::read_dir(canonical)?
        .filter_map(|entry| entry.ok())
        .take(200)
    {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let record = fs::metadata(&path)
            .ok()
            .filter(|metadata| metadata.is_file() && metadata.len() <= MAX_SESSION_RECORD_BYTES)
            .and_then(|_| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<DesignSessionRecord>(&bytes).ok());
        match record {
            Some(record) => records.push(record),
            None => invalid += 1,
        }
    }
    records.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    records.truncate(limit.clamp(1, 50));
    Ok((records, invalid))
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

pub(super) fn read_verified_pixels(
    root: &Path,
    record: &DesignSessionRecord,
) -> Result<VerifiedPixelArtifact> {
    let evidence = record
        .last_evidence
        .as_ref()
        .context("后台设计会话还没有像素证据")?;
    let path = evidence
        .pointer("/artifact/path")
        .and_then(Value::as_str)
        .context("后台设计证据缺少 PNG path")?;
    let expected = evidence
        .pointer("/artifact/sha256")
        .and_then(Value::as_str)
        .context("后台设计证据缺少 PNG sha256")?;
    let media_type = evidence
        .pointer("/artifact/mediaType")
        .and_then(Value::as_str)
        .unwrap_or("image/png");
    if media_type != "image/png" {
        bail!("后台设计像素工件只允许 image/png");
    }
    let path = PathBuf::from(path)
        .canonicalize()
        .context("后台设计 PNG 工件不存在")?;
    let metadata = fs::metadata(&path)?;
    if !path.starts_with(root) || !metadata.is_file() || metadata.len() > MAX_PIXEL_BYTES {
        bail!("后台设计 PNG 越出项目或超过大小上限");
    }
    let bytes = fs::read(path)?;
    let actual = hex::encode(Sha256::digest(&bytes));
    if !expected.eq_ignore_ascii_case(&actual) {
        bail!("后台设计 PNG 哈希不匹配");
    }
    Ok(VerifiedPixelArtifact {
        bytes,
        media_type: media_type.to_string(),
        sha256: actual,
    })
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
