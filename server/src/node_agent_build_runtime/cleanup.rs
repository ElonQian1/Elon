use super::{
    paths::{ensure_removal_tree_safe, ensure_within_root},
    telemetry::directory_size,
};
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CleanupReport {
    pub(crate) reclaimed_bytes: u64,
    pub(crate) removed_paths: usize,
    pub(crate) skipped_active_paths: usize,
}

/// Remove one explicitly selected path below the validated platform root.
///
/// Cache age and disk pressure never call this automatically. Runtime use is
/// limited to the successful task's own temp directory; bulk cache removal is
/// exposed separately through the preview-and-confirm admin API.
pub(crate) fn remove_managed_path(root: &Path, target: &Path) -> Result<u64> {
    ensure_within_root(root, target)?;
    let metadata = match std::fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查构建缓存路径 {}", target.display()));
        }
    };
    ensure_removal_tree_safe(root, target)?;
    if std::fs::canonicalize(root)? == std::fs::canonicalize(target)? {
        anyhow::bail!("拒绝删除统一节点数据根本身");
    }
    let size = directory_size(target);
    if metadata.is_file() {
        std::fs::remove_file(target)
            .with_context(|| format!("无法删除构建缓存文件 {}", target.display()))?;
    } else {
        std::fs::remove_dir_all(target)
            .with_context(|| format!("无法删除构建缓存目录 {}", target.display()))?;
    }
    Ok(size)
}
