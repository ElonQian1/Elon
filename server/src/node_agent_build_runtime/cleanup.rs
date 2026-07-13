use super::{
    admission::{disk_free_bytes, required_free_bytes, BuildCachePolicy},
    lease::active_lease_records,
    paths::{
        ensure_existing_within_root, ensure_removal_tree_safe, ensure_within_root,
        is_link_or_reparse, BuildRunPaths,
    },
    telemetry::{directory_size, unix_now},
};
use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CleanupReport {
    pub(crate) reclaimed_bytes: u64,
    pub(crate) removed_paths: usize,
    pub(crate) skipped_active_paths: usize,
}

impl CleanupReport {
    pub(crate) fn merge(&mut self, other: Self) {
        self.reclaimed_bytes = self.reclaimed_bytes.saturating_add(other.reclaimed_bytes);
        self.removed_paths += other.removed_paths;
        self.skipped_active_paths += other.skipped_active_paths;
    }
}

pub(crate) fn cleanup_expired(
    paths: &BuildRunPaths,
    policy: &BuildCachePolicy,
) -> Result<CleanupReport> {
    let active = active_lease_records(&paths.lease_root)?;
    let active_tasks = active
        .iter()
        .map(|record| elon_pc_dev_runtime::safe_path_part(&record.task_id, "task", 96))
        .collect::<HashSet<_>>();
    let active_rust = active
        .iter()
        .map(|record| (record.project_key.clone(), record.toolchain_key.clone()))
        .collect::<HashSet<_>>();
    let mut report = CleanupReport::default();
    cleanup_expired_children(
        &paths.root,
        &paths.root.join("temp"),
        policy.temp_ttl_secs,
        |path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| active_tasks.contains(name))
        },
        &mut report,
    )?;
    cleanup_expired_rust_targets(paths, policy.cache_ttl_secs, &active_rust, &mut report)?;
    if active.is_empty() {
        for candidate in shared_cache_candidates(paths)? {
            if unix_now().saturating_sub(candidate.modified_at) > policy.cache_ttl_secs {
                remove_candidate(&paths.root, &candidate.path, &mut report)?;
            }
        }
    }
    Ok(report)
}

pub(crate) fn cleanup_for_pressure(
    paths: &BuildRunPaths,
    policy: &BuildCachePolicy,
) -> Result<CleanupReport> {
    let active = active_lease_records(&paths.lease_root)?;
    let active_rust = active
        .iter()
        .map(|record| (record.project_key.clone(), record.toolchain_key.clone()))
        .collect::<HashSet<_>>();
    let mut candidates = rust_target_candidates(paths, &active_rust)?;
    if active.is_empty() {
        candidates.extend(shared_cache_candidates(paths)?);
    }
    candidates.sort_by_key(|candidate| candidate.modified_at);

    let mut report = CleanupReport::default();
    for candidate in candidates {
        if !pressure_remains(paths, policy) {
            break;
        }
        remove_candidate(&paths.root, &candidate.path, &mut report)?;
    }
    Ok(report)
}

pub(crate) fn remove_managed_path(root: &Path, target: &Path) -> Result<u64> {
    ensure_within_root(root, target)?;
    let metadata = match std::fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(error).with_context(|| format!("无法检查构建缓存路径 {}", target.display()));
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

#[derive(Debug)]
struct CleanupCandidate {
    path: PathBuf,
    modified_at: u64,
}

fn cleanup_expired_children(
    root: &Path,
    parent: &Path,
    ttl_secs: u64,
    is_active: impl Fn(&Path) -> bool,
    report: &mut CleanupReport,
) -> Result<()> {
    match std::fs::symlink_metadata(parent) {
        Ok(_) => ensure_existing_within_root(root, parent)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("无法检查待清理目录 {}", parent.display()));
        }
    }
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("无法枚举待清理目录 {}", parent.display()));
        }
    };
    for entry in entries {
        let path = entry
            .with_context(|| format!("无法读取待清理目录项 {}", parent.display()))?
            .path();
        ensure_existing_within_root(root, &path)?;
        if is_active(&path) {
            report.skipped_active_paths += 1;
        } else if is_expired(&path, ttl_secs) {
            remove_candidate(root, &path, report)?;
        }
    }
    Ok(())
}

fn cleanup_expired_rust_targets(
    paths: &BuildRunPaths,
    ttl_secs: u64,
    active: &HashSet<(String, String)>,
    report: &mut CleanupReport,
) -> Result<()> {
    let rust_root = paths.root.join("cache").join("rust-targets");
    match std::fs::symlink_metadata(&rust_root) {
        Ok(_) => ensure_existing_within_root(&paths.root, &rust_root)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查 Rust 构建缓存 {}", rust_root.display()));
        }
    }
    let projects = match std::fs::read_dir(&rust_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法枚举 Rust 构建缓存 {}", rust_root.display()));
        }
    };
    for project in projects {
        let project = project
            .with_context(|| format!("无法读取 Rust 项目缓存项 {}", rust_root.display()))?;
        ensure_existing_within_root(&paths.root, &project.path())?;
        let project_key = project.file_name().to_string_lossy().to_string();
        let toolchains = std::fs::read_dir(project.path())
            .with_context(|| format!("无法枚举 Rust 项目缓存 {}", project.path().display()))?;
        for toolchain in toolchains {
            let toolchain = toolchain.with_context(|| {
                format!("无法读取 Rust toolchain 缓存项 {}", project.path().display())
            })?;
            ensure_existing_within_root(&paths.root, &toolchain.path())?;
            let toolchain_key = toolchain.file_name().to_string_lossy().to_string();
            if active.contains(&(project_key.clone(), toolchain_key)) {
                report.skipped_active_paths += 1;
            } else if is_expired(&toolchain.path(), ttl_secs) {
                remove_candidate(&paths.root, &toolchain.path(), report)?;
            }
        }
    }
    Ok(())
}

fn rust_target_candidates(
    paths: &BuildRunPaths,
    active: &HashSet<(String, String)>,
) -> Result<Vec<CleanupCandidate>> {
    let mut candidates = Vec::new();
    let root = paths.root.join("cache").join("rust-targets");
    match std::fs::symlink_metadata(&root) {
        Ok(_) => ensure_existing_within_root(&paths.root, &root)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(candidates),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查 Rust 构建缓存 {}", root.display()));
        }
    }
    let projects = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(candidates),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法枚举 Rust 构建缓存 {}", root.display()));
        }
    };
    for project in projects {
        let project = project
            .with_context(|| format!("无法读取 Rust 项目缓存项 {}", root.display()))?;
        ensure_existing_within_root(&paths.root, &project.path())?;
        let project_key = project.file_name().to_string_lossy().to_string();
        let toolchains = std::fs::read_dir(project.path())
            .with_context(|| format!("无法枚举 Rust 项目缓存 {}", project.path().display()))?;
        for toolchain in toolchains {
            let toolchain = toolchain.with_context(|| {
                format!("无法读取 Rust toolchain 缓存项 {}", project.path().display())
            })?;
            ensure_existing_within_root(&paths.root, &toolchain.path())?;
            let toolchain_key = toolchain.file_name().to_string_lossy().to_string();
            if !active.contains(&(project_key.clone(), toolchain_key)) {
                candidates.push(CleanupCandidate {
                    modified_at: latest_modified(&toolchain.path()),
                    path: toolchain.path(),
                });
            }
        }
    }
    Ok(candidates)
}

fn shared_cache_candidates(paths: &BuildRunPaths) -> Result<Vec<CleanupCandidate>> {
    let cargo_home = &paths.cargo_home;
    let candidates = [
        cargo_home.join("registry").join("cache"),
        cargo_home.join("registry").join("src"),
        cargo_home.join("git").join("checkouts"),
        paths.gradle_home.join("caches"),
        paths.gradle_home.join("wrapper").join("dists"),
        paths.npm_cache.clone(),
        paths.pnpm_store.clone(),
        paths.corepack_home.clone(),
    ]
    .into_iter()
    .filter(|path| path.exists())
    .collect::<Vec<_>>();
    for path in &candidates {
        ensure_existing_within_root(&paths.root, path)?;
    }
    Ok(candidates
        .into_iter()
        .map(|path| CleanupCandidate {
            modified_at: latest_modified(&path),
            path,
        })
        .collect())
}

fn pressure_remains(paths: &BuildRunPaths, policy: &BuildCachePolicy) -> bool {
    directory_size(&paths.cache_root) > policy.max_total_cache_bytes
        || directory_size(&paths.project_rust_root) > policy.max_project_rust_bytes
        || disk_free_bytes(&paths.root)
            .is_some_and(|bytes| bytes < required_free_bytes(policy))
}

fn remove_candidate(root: &Path, path: &Path, report: &mut CleanupReport) -> Result<()> {
    let reclaimed = remove_managed_path(root, path)?;
    report.reclaimed_bytes = report.reclaimed_bytes.saturating_add(reclaimed);
    report.removed_paths += 1;
    Ok(())
}

fn is_expired(path: &Path, ttl_secs: u64) -> bool {
    unix_now().saturating_sub(latest_modified(path)) > ttl_secs
}

fn latest_modified(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    let own = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return own;
    }
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| latest_modified(&entry.path()))
        .fold(own, u64::max)
}
