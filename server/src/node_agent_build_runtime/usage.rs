use super::{
    paths::{ensure_existing_within_root, BuildRunPaths},
    telemetry::unix_now,
};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Records actual target use separately from artifact mtimes. Cache hits often
/// do not modify target files, so filesystem mtime alone is not an LRU signal.
pub(crate) fn touch(paths: &BuildRunPaths) -> Result<()> {
    let target = target_usage_path(paths, &paths.project_key, &paths.toolchain_key);
    let parent = target.parent().expect("usage path has a parent");
    std::fs::create_dir_all(parent)
        .with_context(|| format!("无法创建 Rust target 使用记录目录 {}", parent.display()))?;
    ensure_existing_within_root(&paths.root, parent)?;
    let payload = format!("{}\n", unix_now());
    crate::node_agent_atomic_file::write(&target, payload.as_bytes())
        .with_context(|| format!("无法更新 Rust target 使用记录 {}", target.display()))?;
    ensure_existing_within_root(&paths.root, &target)?;
    Ok(())
}

pub(crate) fn last_used(
    paths: &BuildRunPaths,
    project_key: &str,
    toolchain_key: &str,
) -> Option<u64> {
    std::fs::read_to_string(target_usage_path(paths, project_key, toolchain_key))
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

pub(crate) fn target_usage_path(
    paths: &BuildRunPaths,
    project_key: &str,
    toolchain_key: &str,
) -> PathBuf {
    paths
        .usage_root
        .join("rust-targets")
        .join(project_key)
        .join(format!("{toolchain_key}.last-used"))
}

pub(crate) fn remove(path: Option<&Path>) {
    if let Some(path) = path {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(
                error = %error,
                path = %path.display(),
                "清理 Rust target 使用记录失败"
            ),
        }
    }
}
