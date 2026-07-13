use super::{
    admission::{disk_free_bytes, disk_total_bytes, required_free_bytes, BuildCachePolicy},
    cleanup::CleanupReport,
    lease::active_lease_count,
    paths::{is_link_or_reparse, BuildRunPaths},
};
use elon_pc_dev_runtime::NodeDataPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct BuildCacheTelemetry {
    pub(crate) root: String,
    pub(crate) project_key: String,
    pub(crate) toolchain_key: String,
    pub(crate) cache_bytes: u64,
    pub(crate) project_rust_bytes: u64,
    pub(crate) temp_bytes: u64,
    pub(crate) disk_free_bytes: Option<u64>,
    pub(crate) active_leases: usize,
    pub(crate) reclaimed_bytes: u64,
    pub(crate) pressure: bool,
    pub(crate) captured_at_unix_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct NodeBuildCacheStatus {
    pub(crate) root: String,
    pub(crate) cache_bytes: u64,
    pub(crate) temp_bytes: u64,
    pub(crate) largest_project_rust_bytes: u64,
    pub(crate) disk_free_bytes: Option<u64>,
    pub(crate) disk_total_bytes: Option<u64>,
    pub(crate) min_free_bytes: u64,
    pub(crate) build_headroom_bytes: u64,
    pub(crate) max_total_cache_bytes: u64,
    pub(crate) max_project_rust_bytes: u64,
    pub(crate) pressure: bool,
    pub(crate) active_leases: usize,
    pub(crate) last_cleanup_at_unix_secs: Option<u64>,
    pub(crate) last_cleanup_reclaimed_bytes: u64,
    pub(crate) captured_at_unix_secs: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CleanupTelemetry {
    cleaned_at_unix_secs: u64,
    reclaimed_bytes: u64,
    removed_paths: usize,
    skipped_active_paths: usize,
}

pub(crate) fn capture(
    paths: &BuildRunPaths,
    reclaimed_bytes: u64,
    policy: &BuildCachePolicy,
) -> BuildCacheTelemetry {
    let disk_free = disk_free_bytes(&paths.root);
    let cache_bytes = directory_size(&paths.cache_root);
    let project_rust_bytes = directory_size(&paths.project_rust_root);
    BuildCacheTelemetry {
        root: paths.root.to_string_lossy().to_string(),
        project_key: paths.project_key.clone(),
        toolchain_key: paths.toolchain_key.clone(),
        cache_bytes,
        project_rust_bytes,
        temp_bytes: directory_size(&paths.root.join("temp")),
        disk_free_bytes: disk_free,
        active_leases: active_lease_count(&paths.lease_root),
        reclaimed_bytes,
        pressure: cache_bytes > policy.max_total_cache_bytes
            || project_rust_bytes > policy.max_project_rust_bytes
            || disk_free.is_some_and(|bytes| bytes < required_free_bytes(policy)),
        captured_at_unix_secs: unix_now(),
    }
}

pub(crate) fn persist(paths: &BuildRunPaths, telemetry: &BuildCacheTelemetry) {
    let target = paths.telemetry_root.join(format!(
        "{}-{}.json",
        paths.project_key, paths.toolchain_key
    ));
    if let Ok(payload) = serde_json::to_vec_pretty(telemetry) {
        let temporary = target.with_extension(format!("json.tmp-{}", uuid::Uuid::new_v4()));
        if std::fs::write(&temporary, payload).is_ok() {
            let _ = std::fs::remove_file(&target);
            let _ = std::fs::rename(temporary, target);
        }
    }
}

pub(crate) fn record_cleanup(paths: &BuildRunPaths, report: &CleanupReport) {
    let telemetry = CleanupTelemetry {
        cleaned_at_unix_secs: unix_now(),
        reclaimed_bytes: report.reclaimed_bytes,
        removed_paths: report.removed_paths,
        skipped_active_paths: report.skipped_active_paths,
    };
    let target = paths.telemetry_root.join("cleanup.json");
    if let Ok(payload) = serde_json::to_vec_pretty(&telemetry) {
        let _ = std::fs::write(target, payload);
    }
}

pub(crate) fn capture_root_status(
    data_paths: &NodeDataPaths,
    policy: &BuildCachePolicy,
) -> NodeBuildCacheStatus {
    let cache = data_paths.cache();
    let temp = data_paths.temp();
    let cache_bytes = directory_size(&cache);
    let largest_project_rust_bytes = largest_project_cache(&data_paths.rust_targets());
    let disk_free = disk_free_bytes(data_paths.root());
    let cleanup = std::fs::read_to_string(cache.join(".telemetry").join("cleanup.json"))
        .ok()
        .and_then(|payload| serde_json::from_str::<CleanupTelemetry>(&payload).ok());
    NodeBuildCacheStatus {
        root: data_paths.root().to_string_lossy().to_string(),
        cache_bytes,
        temp_bytes: directory_size(&temp),
        largest_project_rust_bytes,
        disk_free_bytes: disk_free,
        disk_total_bytes: disk_total_bytes(data_paths.root()),
        min_free_bytes: policy.min_free_bytes,
        build_headroom_bytes: policy.build_headroom_bytes,
        max_total_cache_bytes: policy.max_total_cache_bytes,
        max_project_rust_bytes: policy.max_project_rust_bytes,
        pressure: cache_bytes > policy.max_total_cache_bytes
            || largest_project_rust_bytes > policy.max_project_rust_bytes
            || disk_free.is_some_and(|bytes| bytes < required_free_bytes(policy)),
        active_leases: active_lease_count(&cache.join(".leases")),
        last_cleanup_at_unix_secs: cleanup.as_ref().map(|status| status.cleaned_at_unix_secs),
        last_cleanup_reclaimed_bytes: cleanup
            .as_ref()
            .map(|status| status.reclaimed_bytes)
            .unwrap_or_default(),
        captured_at_unix_secs: unix_now(),
    }
}

fn largest_project_cache(rust_targets: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(rust_targets) else {
        return 0;
    };
    if !metadata.is_dir() || is_link_or_reparse(&metadata) {
        return 0;
    }
    std::fs::read_dir(rust_targets)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| directory_size(&entry.path()))
        .max()
        .unwrap_or_default()
}

pub(crate) fn directory_size(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if is_link_or_reparse(&metadata) {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| directory_size(&entry.path()))
        .fold(0_u64, u64::saturating_add)
}

pub(crate) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
