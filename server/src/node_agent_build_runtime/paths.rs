use anyhow::{anyhow, Context, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildRunPaths {
    pub(crate) root: PathBuf,
    pub(crate) cache_root: PathBuf,
    pub(crate) project_rust_root: PathBuf,
    pub(crate) cargo_target: PathBuf,
    pub(crate) cargo_home: PathBuf,
    pub(crate) gradle_home: PathBuf,
    pub(crate) npm_cache: PathBuf,
    pub(crate) pnpm_store: PathBuf,
    pub(crate) corepack_home: PathBuf,
    pub(crate) task_temp: PathBuf,
    pub(crate) lease_root: PathBuf,
    pub(crate) telemetry_root: PathBuf,
    pub(crate) project_key: String,
    pub(crate) toolchain_key: String,
}

pub(crate) fn resolve_run_paths(
    paths: &NodeDataPaths,
    task_id: &str,
    project_id: &str,
    cwd: Option<&Path>,
) -> Result<BuildRunPaths> {
    require_absolute_root(paths.root())?;
    let project_key = elon_pc_dev_runtime::safe_path_part(project_id, "project", 96);
    let toolchain_key = rust_toolchain_key(cwd);
    let cache_root = paths.cache();
    Ok(BuildRunPaths {
        root: paths.root().to_path_buf(),
        cache_root: cache_root.clone(),
        project_rust_root: paths.rust_targets().join(&project_key),
        cargo_target: paths.project_rust_target(&project_key, &toolchain_key),
        cargo_home: paths.cargo_home(),
        gradle_home: paths.gradle_home(),
        npm_cache: paths.npm_cache(),
        pnpm_store: paths.pnpm_store(),
        corepack_home: cache_root.join("corepack"),
        task_temp: paths.task_temp(task_id),
        lease_root: cache_root.join(".leases"),
        telemetry_root: cache_root.join(".telemetry"),
        project_key,
        toolchain_key,
    })
}

pub(crate) fn prepare_run_directories(paths: &BuildRunPaths) -> Result<()> {
    for directory in [
        &paths.cache_root,
        &paths.project_rust_root,
        &paths.cargo_target,
        &paths.cargo_home,
        &paths.gradle_home,
        &paths.npm_cache,
        &paths.pnpm_store,
        &paths.corepack_home,
        &paths.task_temp,
        &paths.lease_root,
        &paths.telemetry_root,
    ] {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("无法创建 PC 节点构建目录 {}", directory.display()))?;
        ensure_within_root(&paths.root, directory)?;
    }
    Ok(())
}

pub(crate) fn ensure_within_root(root: &Path, candidate: &Path) -> Result<()> {
    let root = absolute_lexical(root)?;
    let candidate = absolute_lexical(candidate)?;
    if candidate == root || candidate.starts_with(&root) {
        Ok(())
    } else {
        Err(anyhow!(
            "拒绝访问统一节点数据根之外的构建路径: {}",
            candidate.display()
        ))
    }
}

fn require_absolute_root(root: &Path) -> Result<()> {
    if !root.is_absolute() {
        return Err(anyhow!(
            "ELON_NODE_DATA_ROOT 必须是绝对路径，当前值: {}",
            root.display()
        ));
    }
    Ok(())
}

fn absolute_lexical(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(anyhow!("路径不是绝对路径: {}", path.display()));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return Err(anyhow!("路径越过文件系统根: {}", path.display()));
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn rust_toolchain_key(cwd: Option<&Path>) -> String {
    let explicit = std::env::var("RUSTUP_TOOLCHAIN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let detected = explicit.or_else(|| rustup_active_toolchain(cwd));
    let identity =
        detected.unwrap_or_else(|| rustc_identity(cwd).unwrap_or_else(|| "default".into()));
    let readable = elon_pc_dev_runtime::safe_path_part(&identity, "default", 36);
    let digest = Sha256::digest(identity.as_bytes());
    format!("{}-{:x}", readable, digest)[..readable.len() + 13].to_string()
}

fn rustup_active_toolchain(cwd: Option<&Path>) -> Option<String> {
    let mut command = std::process::Command::new("rustup");
    command.args(["show", "active-toolchain"]);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next()
            .unwrap_or("default")
            .to_string()
    })
}

fn rustc_identity(cwd: Option<&Path>) -> Option<String> {
    let mut command = std::process::Command::new("rustc");
    command.args(["--version", "--verbose"]);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}
