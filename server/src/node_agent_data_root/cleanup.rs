use anyhow::{bail, Context, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CleanupEntry {
    pub(crate) kind: &'static str,
    pub(crate) path: String,
    pub(crate) existed: bool,
    pub(crate) estimated_bytes: u64,
    pub(crate) removed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CleanupResult {
    pub(crate) apply: bool,
    pub(crate) estimated_bytes: u64,
    pub(crate) entries: Vec<CleanupEntry>,
}

pub(crate) fn cleanup(
    paths: &NodeDataPaths,
    expected_install_id: &str,
    apply: bool,
) -> Result<CleanupResult> {
    // Carry the live runtime owner through the complete deletion transaction.
    // Accepting an arbitrary well-formed marker here would leave a window in
    // which a replaced root could authorize deletion with another node's ID.
    super::verify_root_marker(paths, expected_install_id)?;
    let canonical_root = super::validate_created_root(paths.root())?;
    let targets = [("cache", paths.cache()), ("temp", paths.temp())];
    let mut entries = Vec::with_capacity(targets.len());
    for (kind, target) in targets {
        super::validate_canonical_managed_path(paths.root(), &canonical_root, &target)?;
        let existed = target.exists();
        let estimated_bytes = directory_size_without_following_links(&target)?;
        if apply && existed {
            super::verify_root_marker(paths, expected_install_id)?;
            // Re-scan immediately before deletion. This rejects a junction or
            // symlink anywhere in the managed tree instead of allowing
            // remove_dir_all to cross an out-of-root reparse boundary.
            validate_tree_no_reparse(&target)?;
            super::validate_canonical_managed_path(paths.root(), &canonical_root, &target)?;
            std::fs::remove_dir_all(&target)
                .with_context(|| format!("无法清理节点 {kind} 目录 {}", target.display()))?;
            std::fs::create_dir_all(&target)
                .with_context(|| format!("无法重建节点 {kind} 目录 {}", target.display()))?;
            super::verify_root_marker(paths, expected_install_id)?;
            super::validate_canonical_managed_path(paths.root(), &canonical_root, &target)?;
        }
        entries.push(CleanupEntry {
            kind,
            path: super::path_text(&target),
            existed,
            estimated_bytes,
            removed: apply && existed,
        });
    }
    Ok(CleanupResult {
        apply,
        estimated_bytes: entries.iter().map(|entry| entry.estimated_bytes).sum(),
        entries,
    })
}

fn directory_size_without_following_links(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if super::metadata_is_reparse_point(&metadata) {
        bail!(
            "拒绝读取包含符号链接、junction 或重解析点的清理目录: {}",
            path.display()
        );
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        total = total.saturating_add(directory_size_without_following_links(&entry.path())?);
    }
    Ok(total)
}

fn validate_tree_no_reparse(path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("无法检查待清理目录 {}", path.display()));
        }
    };
    if super::metadata_is_reparse_point(&metadata) {
        bail!(
            "拒绝清理包含符号链接、junction 或重解析点的目录: {}",
            path.display()
        );
    }
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path)
            .with_context(|| format!("无法枚举待清理目录 {}", path.display()))?
        {
            validate_tree_no_reparse(&entry?.path())?;
        }
    }
    Ok(())
}
