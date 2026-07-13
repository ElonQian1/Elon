use super::{
    cleanup::{cleanup_expired, cleanup_for_pressure, CleanupReport},
    paths::BuildRunPaths,
    reservation,
    telemetry::directory_size,
};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct BuildCachePolicy {
    pub(crate) min_free_bytes: u64,
    pub(crate) build_headroom_bytes: u64,
    pub(crate) max_total_cache_bytes: u64,
    pub(crate) max_project_rust_bytes: u64,
    pub(crate) temp_ttl_secs: u64,
    pub(crate) cache_ttl_secs: u64,
}

impl Default for BuildCachePolicy {
    fn default() -> Self {
        Self {
            min_free_bytes: env_u64("ELON_NODE_BUILD_MIN_FREE_BYTES").unwrap_or(10 * GIB),
            build_headroom_bytes: env_u64_allow_zero("ELON_NODE_BUILD_HEADROOM_BYTES")
                .unwrap_or(24 * GIB),
            max_total_cache_bytes: env_u64("ELON_NODE_BUILD_MAX_CACHE_BYTES").unwrap_or(80 * GIB),
            max_project_rust_bytes: env_u64("ELON_NODE_BUILD_MAX_PROJECT_RUST_BYTES")
                .unwrap_or(24 * GIB),
            temp_ttl_secs: env_u64("ELON_NODE_BUILD_TEMP_TTL_SECS").unwrap_or(24 * 60 * 60),
            cache_ttl_secs: env_u64("ELON_NODE_BUILD_CACHE_TTL_SECS").unwrap_or(30 * 24 * 60 * 60),
        }
    }
}

pub(crate) fn admit(
    paths: &BuildRunPaths,
    policy: &BuildCachePolicy,
    active_reserved_bytes: u64,
) -> Result<CleanupReport> {
    let mut report = cleanup_expired(paths, policy)?;
    if under_pressure(paths, policy, active_reserved_bytes) {
        report.merge(cleanup_for_pressure(paths, policy, active_reserved_bytes)?);
    }

    let cache_bytes = directory_size(&paths.cache_root);
    let project_bytes = directory_size(&paths.project_rust_root);
    let free_bytes = disk_free_bytes(&paths.root).ok_or_else(|| {
        anyhow!(
            "无法读取 PC 节点构建盘可用容量，已按 fail-closed 拒绝启动任务: {}",
            paths.root.display()
        )
    })?;
    let required_free = reservation::admission_required_free(policy, active_reserved_bytes);
    if free_bytes < required_free {
        return Err(anyhow!(
            "PC 节点构建盘空间不足：安全底线 {} GiB + 活动任务预留 {} GiB + 本次构建预留 {} GiB，启动前需至少 {} GiB 空闲，当前约 {} GiB",
            policy.min_free_bytes / GIB,
            active_reserved_bytes / GIB,
            policy.build_headroom_bytes / GIB,
            required_free / GIB,
            free_bytes / GIB
        ));
    }
    if cache_bytes > policy.max_total_cache_bytes {
        return Err(anyhow!(
            "PC 节点构建缓存超过总配额：{} GiB / {} GiB",
            cache_bytes / GIB,
            policy.max_total_cache_bytes / GIB
        ));
    }
    if project_bytes > policy.max_project_rust_bytes {
        return Err(anyhow!(
            "项目 Rust 构建缓存超过配额：{} GiB / {} GiB",
            project_bytes / GIB,
            policy.max_project_rust_bytes / GIB
        ));
    }
    Ok(report)
}

pub(crate) fn under_pressure(
    paths: &BuildRunPaths,
    policy: &BuildCachePolicy,
    active_reserved_bytes: u64,
) -> bool {
    directory_size(&paths.cache_root) > policy.max_total_cache_bytes
        || directory_size(&paths.project_rust_root) > policy.max_project_rust_bytes
        || disk_free_bytes(&paths.root)
            .map(|bytes| bytes < reservation::cleanup_required_free(policy, active_reserved_bytes))
            .unwrap_or(false)
}

pub(crate) fn required_free_bytes(policy: &BuildCachePolicy) -> u64 {
    reservation::admission_required_free(policy, 0)
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
}

fn env_u64_allow_zero(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

pub(crate) fn disk_free_bytes(path: &Path) -> Option<u64> {
    let target = existing_ancestor(path)?;
    disk_free_bytes_platform(&target)
}

pub(crate) fn disk_total_bytes(path: &Path) -> Option<u64> {
    let target = existing_ancestor(path)?;
    disk_total_bytes_platform(&target)
}

fn existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();
    loop {
        if current.exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

#[cfg(windows)]
fn disk_free_bytes_platform(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut free = 0_u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free)
}

#[cfg(windows)]
fn disk_total_bytes_platform(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    let wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut total = 0_u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            &mut total,
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(total)
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetDiskFreeSpaceExW(
        directory_name: *const u16,
        free_bytes_available: *mut u64,
        total_number_of_bytes: *mut u64,
        total_number_of_free_bytes: *mut u64,
    ) -> i32;
}

#[cfg(not(windows))]
fn disk_free_bytes_platform(path: &Path) -> Option<u64> {
    let output = std::process::Command::new("df")
        .args(["-Pk"])
        .arg(path)
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .nth(1)?
        .to_string();
    line.split_whitespace()
        .nth(3)?
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)
}

#[cfg(not(windows))]
fn disk_total_bytes_platform(path: &Path) -> Option<u64> {
    let output = std::process::Command::new("df")
        .args(["-Pk"])
        .arg(path)
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .nth(1)?
        .to_string();
    line.split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)
}
