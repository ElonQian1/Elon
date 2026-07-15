use super::{cleanup::CleanupReport, paths::BuildRunPaths, reservation, telemetry::directory_size};
use anyhow::Result;
use std::path::{Path, PathBuf};

const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct BuildCachePolicy {
    pub(crate) min_free_bytes: u64,
    pub(crate) build_headroom_bytes: u64,
    pub(crate) max_total_cache_bytes: u64,
    pub(crate) max_project_rust_bytes: u64,
}

impl Default for BuildCachePolicy {
    fn default() -> Self {
        Self {
            min_free_bytes: env_u64("ELON_NODE_BUILD_MIN_FREE_BYTES").unwrap_or(4 * GIB),
            build_headroom_bytes: env_u64_allow_zero("ELON_NODE_BUILD_HEADROOM_BYTES")
                .unwrap_or(8 * GIB),
            max_total_cache_bytes: env_u64("ELON_NODE_BUILD_MAX_CACHE_BYTES").unwrap_or(80 * GIB),
            max_project_rust_bytes: env_u64("ELON_NODE_BUILD_MAX_PROJECT_RUST_BYTES")
                .unwrap_or(24 * GIB),
        }
    }
}

pub(crate) fn admit(
    paths: &BuildRunPaths,
    policy: &BuildCachePolicy,
    active_reserved_bytes: u64,
) -> Result<CleanupReport> {
    for advisory in advisories(paths, policy, active_reserved_bytes) {
        tracing::warn!(%advisory, "项目数据架构体检建议；不阻止任务、不自动清理缓存");
    }
    Ok(CleanupReport::default())
}

pub(crate) fn advisories(
    paths: &BuildRunPaths,
    policy: &BuildCachePolicy,
    active_reserved_bytes: u64,
) -> Vec<String> {
    let mut warnings = Vec::new();
    let cache_bytes = directory_size(&paths.cache_root);
    let project_bytes = directory_size(&paths.project_rust_root);
    let required_free = reservation::admission_required_free(policy, active_reserved_bytes);
    match disk_free_bytes(&paths.root) {
        Some(free_bytes) if free_bytes < required_free => warnings.push(format!(
            "AI 临时工作区剩余约 {} GiB，低于建议余量 {} GiB；任务继续运行，AI 可优先复用旧缓存或提出整理方案",
            free_bytes / GIB,
            required_free / GIB
        )),
        None => warnings.push(format!(
            "暂时无法读取 {} 的可用容量；任务继续运行，实际构建结果作为判断依据",
            paths.root.display()
        )),
        _ => {}
    }
    if cache_bytes > policy.max_total_cache_bytes {
        warnings.push(format!(
            "一龙推荐缓存约 {} GiB，超过建议值 {} GiB；不会自动删除，可在体检后选择整理",
            cache_bytes / GIB,
            policy.max_total_cache_bytes / GIB
        ));
    }
    if project_bytes > policy.max_project_rust_bytes {
        warnings.push(format!(
            "当前托管项目 Rust 缓存约 {} GiB，超过建议值 {} GiB；不会阻止项目",
            project_bytes / GIB,
            policy.max_project_rust_bytes / GIB
        ));
    }
    warnings
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
