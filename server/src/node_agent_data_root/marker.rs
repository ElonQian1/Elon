use anyhow::{bail, Context, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use std::path::Path;

pub(crate) const ROOT_MARKER_FILE: &str = ".elon-node-data-root.json";
const ROOT_MARKER_SCHEMA_VERSION: u64 = 1;

pub(super) fn claim_or_verify_root_marker(paths: &NodeDataPaths, install_id: &str) -> Result<()> {
    let install_id = require_install_id(install_id)?;
    let marker = paths.root().join(ROOT_MARKER_FILE);
    if let Some(existing_install_id) = read_existing_root_marker(&marker)? {
        return ensure_marker_owner(&marker, &existing_install_id, install_id);
    }

    if !directory_is_empty(paths.root())? {
        // A concurrent claimant may have installed the marker after our first
        // lookup and may already be creating managed roots. Re-read once before
        // treating the directory as an unsafe pre-existing directory.
        if let Some(existing_install_id) = read_existing_root_marker(&marker)? {
            return ensure_marker_owner(&marker, &existing_install_id, install_id);
        }
        bail!(
            "未标记的节点数据根必须是空目录，请选择专用空目录: {}",
            paths.root().display()
        );
    }

    let content = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": ROOT_MARKER_SCHEMA_VERSION,
        "install_id": install_id,
    }))?;
    match crate::node_agent_atomic_file::write_new(&marker, &content) {
        Ok(()) => Ok(()),
        Err(claim_error) => match read_existing_root_marker(&marker) {
            Ok(Some(existing_install_id)) => {
                ensure_marker_owner(&marker, &existing_install_id, install_id)
            }
            Ok(None) => Err(claim_error)
                .with_context(|| format!("无法独占提交节点数据根标记 {}", marker.display())),
            Err(marker_error) => Err(marker_error)
                .with_context(|| format!("节点数据根标记并发提交失败，原始错误: {claim_error:#}")),
        },
    }
}

pub(crate) fn verify_root_marker(paths: &NodeDataPaths, install_id: &str) -> Result<()> {
    let install_id = require_install_id(install_id)?;
    let marker = paths.root().join(ROOT_MARKER_FILE);
    let Some(existing_install_id) = read_existing_root_marker(&marker)? else {
        bail!("节点数据根缺少所有权标记: {}", marker.display());
    };
    if existing_install_id != install_id {
        bail!("该目录已属于另一台一龙节点: {}", marker.display());
    }
    Ok(())
}

/// Validates marker bytes read from an already pinned file handle. Callers that need a
/// time-of-check/time-of-use resistant root must not reopen the marker by path after pinning.
pub(crate) fn verify_root_marker_payload(existing: &str, install_id: &str) -> Result<()> {
    let expected = require_install_id(install_id)?;
    let actual = parse_root_marker_payload(existing)?;
    if actual != expected {
        bail!("节点数据根标记属于另一台一龙节点");
    }
    Ok(())
}

pub(super) fn read_existing_root_marker(marker: &Path) -> Result<Option<String>> {
    let metadata = match std::fs::symlink_metadata(marker) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查节点数据根标记 {}", marker.display()));
        }
    };
    if super::metadata_is_reparse_point(&metadata) {
        bail!(
            "节点数据根标记不能是符号链接、junction 或重解析点: {}",
            marker.display()
        );
    }
    if !metadata.is_file() {
        bail!("节点数据根标记不是普通文件: {}", marker.display());
    }
    let existing = std::fs::read_to_string(marker)
        .with_context(|| format!("无法读取节点数据根标记 {}", marker.display()))?;
    let install_id = parse_root_marker_payload(&existing)
        .with_context(|| format!("节点数据根标记损坏: {}", marker.display()))?;
    Ok(Some(install_id))
}

fn parse_root_marker_payload(existing: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(existing)?;
    let schema_version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("节点数据根标记缺少有效 schema_version"))?;
    if schema_version != ROOT_MARKER_SCHEMA_VERSION {
        bail!(
            "节点数据根标记 schema_version 不受支持 (expected {}, actual {})",
            ROOT_MARKER_SCHEMA_VERSION,
            schema_version
        );
    }
    let install_id = value
        .get("install_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("节点数据根标记缺少有效 install_id"))?;
    Ok(install_id.to_string())
}

pub(super) fn root_marker_belongs_to(root: &Path, install_id: &str) -> bool {
    verify_root_marker(&NodeDataPaths::new(root), install_id).is_ok()
}

fn require_install_id(install_id: &str) -> Result<&str> {
    let install_id = install_id.trim();
    if install_id.is_empty() {
        bail!("节点安装 ID 不能为空，拒绝绑定数据根");
    }
    Ok(install_id)
}

fn ensure_marker_owner(marker: &Path, existing: &str, expected: &str) -> Result<()> {
    if existing != expected {
        bail!("该目录已属于另一台一龙节点: {}", marker.display());
    }
    Ok(())
}

fn directory_is_empty(path: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)
        .with_context(|| format!("无法检查节点数据根是否为空 {}", path.display()))?
        .next()
        .is_none())
}
