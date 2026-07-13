use super::{
    admission::{disk_free_bytes, BuildCachePolicy},
    lease::active_lease_records,
    paths::{ensure_within_root, BuildRunPaths},
    telemetry::{directory_size, unix_now},
};
use anyhow::{Context, Result};
use std::{collections::HashSet, path::{Path, PathBuf}};

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
    let active = active_lease_records(&paths.lease_root);
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
        |path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| active_tasks.contains(name)),
        &mut report,
    )?;
    cleanup_expired_rust_targets(paths, policy.cache_ttl_secs, &active_rust, &mut report)?;
    if active.is_empty() {
        for candidate in shared_cache_candidates(paths) {
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
    let active = active_lease_records(&paths.lease_root);
    let active_rust = active
        .iter()
        .map(|record| (record.project_key.clone(), record.toolchain_key.clone()))
        .collect::<HashSet<_>>();
    let mut candidates = rust_target_candidates(paths, &active_rust)?;
    if active.is_empty() {
        candidates.extend(shared_cache_candidates(paths));
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
    if target == root {
        anyhow::bail!("拒绝删除统一节点数据根本身");
    }
    let size = directory_size(target);
    let Ok(metadata) = std::fs::symlink_metadata(target) else {
        return Ok(0);
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
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
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Ok(());
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
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
    let Ok(projects) = std::fs::read_dir(paths.root.join("cache").join("rust-targets")) else {
        return Ok(());
    };
    for project in projects.filter_map(|entry| entry.ok()) {
        let project_key = project.file_name().to_string_lossy().to_string();
        let Ok(toolchains) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for toolchain in toolchains.filter_map(|entry| entry.ok()) {
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
    let Ok(projects) = std::fs::read_dir(root) else {
        return Ok(candidates);
    };
    for project in projects.filter_map(|entry| entry.ok()) {
        let project_key = project.file_name().to_string_lossy().to_string();
        let Ok(toolchains) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for toolchain in toolchains.filter_map(|entry| entry.ok()) {
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

fn shared_cache_candidates(paths: &BuildRunPaths) -> Vec<CleanupCandidate> {
    let cargo_home = &paths.cargo_home;
    [
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
    .map(|path| CleanupCandidate {
        modified_at: latest_modified(&path),
        path,
    })
    .collect()
}

fn pressure_remains(paths: &BuildRunPaths, policy: &BuildCachePolicy) -> bool {
    directory_size(&paths.cache_root) > policy.max_total_cache_bytes
        || directory_size(&paths.project_rust_root) > policy.max_project_rust_bytes
        || disk_free_bytes(&paths.root).is_some_and(|bytes| bytes < policy.min_free_bytes)
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
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
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
