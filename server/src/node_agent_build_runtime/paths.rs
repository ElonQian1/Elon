use anyhow::{anyhow, Context, Result};
use elon_pc_dev_runtime::NodeDataPaths;
use sha2::{Digest, Sha256};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

const SHORT_DIGEST_LEN: usize = 12;
const MAX_PROJECT_KEY_LEN: usize = 96;
const MAX_TASK_KEY_LEN: usize = 96;

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
    pub(crate) yarn_cache: PathBuf,
    pub(crate) yarn_global: PathBuf,
    pub(crate) node_gyp_cache: PathBuf,
    pub(crate) sccache: PathBuf,
    pub(crate) corepack_home: PathBuf,
    pub(crate) task_temp: PathBuf,
    pub(crate) target_lock_root: PathBuf,
    pub(crate) lease_root: PathBuf,
    pub(crate) usage_root: PathBuf,
    pub(crate) telemetry_root: PathBuf,
    pub(crate) task_key: String,
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
    let task_key = stable_task_key(task_id);
    let project_key = stable_project_key(project_id);
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
        yarn_cache: paths.yarn_cache(),
        yarn_global: paths.yarn_global(),
        node_gyp_cache: paths.node_gyp_cache(),
        sccache: paths.sccache(),
        corepack_home: cache_root.join("corepack"),
        task_temp: paths.task_temp_for_key(&task_key),
        target_lock_root: paths.root().join(".runtime").join("target-locks"),
        lease_root: cache_root.join(".leases"),
        usage_root: cache_root.join(".usage"),
        telemetry_root: cache_root.join(".telemetry"),
        task_key,
        project_key,
        toolchain_key,
    })
}

pub(crate) fn prepare_run_directories(paths: &BuildRunPaths) -> Result<()> {
    prepare_managed_directory(&paths.root, &paths.root)?;
    for directory in [
        &paths.cache_root,
        &paths.project_rust_root,
        &paths.cargo_target,
        &paths.cargo_home,
        &paths.gradle_home,
        &paths.npm_cache,
        &paths.pnpm_store,
        &paths.yarn_cache,
        &paths.yarn_global,
        &paths.node_gyp_cache,
        &paths.sccache,
        &paths.corepack_home,
        &paths.task_temp,
        &paths.target_lock_root,
        &paths.lease_root,
        &paths.usage_root,
        &paths.telemetry_root,
    ] {
        prepare_managed_directory(&paths.root, directory)?;
    }
    Ok(())
}

fn prepare_managed_directory(root: &Path, directory: &Path) -> Result<()> {
    ensure_safe_for_creation(root, directory)?;
    std::fs::create_dir_all(directory)
        .with_context(|| format!("无法创建 PC 节点构建目录 {}", directory.display()))?;
    ensure_existing_within_root(root, directory)
}

pub(crate) fn ensure_within_root(root: &Path, candidate: &Path) -> Result<()> {
    let root = absolute_lexical(root)?;
    let candidate = absolute_lexical(candidate)?;
    if same_path(&candidate, &root) || path_starts_with(&candidate, &root) {
        Ok(())
    } else {
        Err(anyhow!(
            "拒绝访问统一节点数据根之外的构建路径: {}",
            candidate.display()
        ))
    }
}

/// 验证一个已经存在的受管路径没有经过 symlink / Windows reparse point，
/// 并用 canonical path 再确认它的真实位置仍位于统一节点数据根内。
pub(crate) fn ensure_existing_within_root(root: &Path, candidate: &Path) -> Result<()> {
    ensure_within_root(root, candidate)?;
    reject_link_components(root)?;
    reject_link_components(candidate)?;

    let root_metadata = std::fs::symlink_metadata(root)
        .with_context(|| format!("无法读取统一节点数据根 {}", root.display()))?;
    if !root_metadata.is_dir() || is_link_or_reparse(&root_metadata) {
        return Err(anyhow!(
            "统一节点数据根必须是真实目录，不能是 symlink、junction 或 reparse point: {}",
            root.display()
        ));
    }
    let candidate_metadata = std::fs::symlink_metadata(candidate)
        .with_context(|| format!("无法读取受管构建路径 {}", candidate.display()))?;
    if is_link_or_reparse(&candidate_metadata) {
        return Err(anyhow!(
            "受管构建路径不能是 symlink、junction 或 reparse point: {}",
            candidate.display()
        ));
    }

    let real_root = std::fs::canonicalize(root)
        .with_context(|| format!("无法解析统一节点数据根真实路径 {}", root.display()))?;
    let real_candidate = std::fs::canonicalize(candidate)
        .with_context(|| format!("无法解析受管构建路径真实位置 {}", candidate.display()))?;
    if same_path(&real_candidate, &real_root) || path_starts_with(&real_candidate, &real_root) {
        Ok(())
    } else {
        Err(anyhow!(
            "拒绝访问真实位置位于统一节点数据根之外的构建路径: {} -> {}",
            candidate.display(),
            real_candidate.display()
        ))
    }
}

/// 删除目录树前逐项检查，任何嵌套的 symlink / junction / reparse point 都会让
/// 清理 fail closed。这样不会把 `remove_dir_all` 的平台差异变成越界删除风险。
pub(crate) fn ensure_removal_tree_safe(root: &Path, target: &Path) -> Result<()> {
    ensure_existing_within_root(root, target)?;
    reject_tree_links(target)
}

fn reject_tree_links(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("无法检查待清理路径 {}", path.display()))?;
    if is_link_or_reparse(&metadata) {
        return Err(anyhow!(
            "待清理目录树包含 symlink、junction 或 reparse point，拒绝删除: {}",
            path.display()
        ));
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let entries = std::fs::read_dir(path)
        .with_context(|| format!("无法枚举待清理目录 {}", path.display()))?;
    for entry in entries {
        reject_tree_links(
            &entry
                .with_context(|| format!("无法读取待清理目录项 {}", path.display()))?
                .path(),
        )?;
    }
    Ok(())
}

/// 创建前的 fail-closed 检查。路径可以尚不存在，但所有已经存在的父级都必须
/// 是普通目录，避免 `create_dir_all` 沿 junction / symlink 写出数据根。
fn ensure_safe_for_creation(root: &Path, candidate: &Path) -> Result<()> {
    ensure_within_root(root, candidate)?;
    reject_link_components(root)?;
    reject_link_components(candidate)
}

fn reject_link_components(path: &Path) -> Result<()> {
    let normalized = absolute_lexical(path)?;
    let mut current = PathBuf::new();
    for component in normalized.components() {
        current.push(component.as_os_str());
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if is_link_or_reparse(&metadata) => {
                return Err(anyhow!(
                    "路径包含 symlink、junction 或 reparse point，拒绝使用: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("无法检查受管路径组件 {}", current.display()));
            }
        }
    }
    Ok(())
}

pub(crate) fn is_link_or_reparse(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
fn same_path(left: &Path, right: &Path) -> bool {
    left.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
}

#[cfg(not(windows))]
fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}

#[cfg(windows)]
fn path_starts_with(path: &Path, base: &Path) -> bool {
    let path = path.components().collect::<Vec<_>>();
    let base = base.components().collect::<Vec<_>>();
    path.len() >= base.len()
        && path.iter().zip(base.iter()).all(|(left, right)| {
            left.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
        })
}

#[cfg(not(windows))]
fn path_starts_with(path: &Path, base: &Path) -> bool {
    path.starts_with(base)
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
    let mut normal_depth = 0_usize;
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if normal_depth == 0 {
                    return Err(anyhow!("路径越过文件系统根: {}", path.display()));
                }
                normalized.pop();
                normal_depth -= 1;
            }
            std::path::Component::Normal(value) => {
                normalized.push(value);
                normal_depth += 1;
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn stable_project_key(project_id: &str) -> String {
    let readable_limit = MAX_PROJECT_KEY_LEN - SHORT_DIGEST_LEN - 1;
    let readable = elon_pc_dev_runtime::safe_path_part(project_id, "project", readable_limit);
    let digest = format!("{:x}", Sha256::digest(project_id.as_bytes()));
    format!("{readable}-{}", &digest[..SHORT_DIGEST_LEN])
}

pub(crate) fn stable_task_key(task_id: &str) -> String {
    let readable_limit = MAX_TASK_KEY_LEN - SHORT_DIGEST_LEN - 1;
    let readable = elon_pc_dev_runtime::safe_path_part(task_id, "task", readable_limit);
    let digest = format!("{:x}", Sha256::digest(task_id.as_bytes()));
    format!("{readable}-{}", &digest[..SHORT_DIGEST_LEN])
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
