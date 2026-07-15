use super::{
    paths::{ensure_existing_within_root, BuildRunPaths},
    telemetry::unix_now,
};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Records actual target use separately from artifact mtimes for architecture
/// advice and future user-authorized migration. This metadata never deletes a
/// cache or blocks a task by itself.
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
