use super::{
    paths::{ensure_existing_within_root, stable_task_key, BuildRunPaths},
    telemetry::unix_now,
};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

#[cfg(windows)]
use std::ffi::c_void;

pub(crate) const ACTIVE_LEASE_TTL_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LeaseRecord {
    pub(crate) task_id: String,
    #[serde(default)]
    pub(crate) task_key: String,
    pub(crate) project_key: String,
    pub(crate) toolchain_key: String,
    pub(crate) temp_path: String,
    pub(crate) created_at_unix_secs: u64,
    #[serde(default)]
    pub(crate) process_id: u32,
    #[serde(default)]
    pub(crate) runtime_id: String,
    #[serde(default)]
    pub(crate) reserved_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct BuildRunLease {
    root: PathBuf,
    path: PathBuf,
    task_key: String,
    runtime_id: String,
}

impl BuildRunLease {
    pub(crate) fn acquire(
        paths: &BuildRunPaths,
        task_id: &str,
        reserved_bytes: u64,
    ) -> Result<Self> {
        std::fs::create_dir_all(&paths.lease_root)?;
        ensure_existing_within_root(&paths.root, &paths.lease_root)?;
        let path = paths
            .lease_root
            .join(format!("{}.lease.json", paths.task_key));
        if path.exists() {
            ensure_existing_within_root(&paths.root, &path)?;
            if lease_is_stale(&path)? {
                remove_file_if_present(&path)
                    .with_context(|| format!("无法移除过期构建任务 lease: {}", path.display()))?;
            } else {
                bail!("构建任务 lease 已存在且仍活动: {}", path.display());
            }
        }

        register_active_path(&path)?;
        let current_runtime_id = runtime_id().to_string();
        let record = LeaseRecord {
            task_id: task_id.to_string(),
            task_key: paths.task_key.clone(),
            project_key: paths.project_key.clone(),
            toolchain_key: paths.toolchain_key.clone(),
            temp_path: paths.task_temp.to_string_lossy().to_string(),
            created_at_unix_secs: unix_now(),
            process_id: std::process::id(),
            runtime_id: current_runtime_id.clone(),
            reserved_bytes,
        };
        let publish_result = (|| -> Result<()> {
            let mut payload = serde_json::to_vec_pretty(&record)?;
            payload.push(b'\n');
            crate::node_agent_atomic_file::write_new(&path, &payload)
                .with_context(|| format!("无法原子发布构建任务 lease: {}", path.display()))?;
            ensure_existing_within_root(&paths.root, &path)?;
            Ok(())
        })();

        if let Err(error) = publish_result {
            remove_owned_lease(&path, &current_runtime_id, &paths.task_key);
            unregister_active_path(&path);
            return Err(error);
        }

        Ok(Self {
            root: paths.root.clone(),
            path,
            task_key: paths.task_key.clone(),
            runtime_id: current_runtime_id,
        })
    }
}

impl Drop for BuildRunLease {
    fn drop(&mut self) {
        if ensure_existing_within_root(&self.root, &self.path).is_ok() {
            remove_owned_lease(&self.path, &self.runtime_id, &self.task_key);
        }
        unregister_active_path(&self.path);
    }
}

pub(crate) fn active_lease_count(lease_root: &Path) -> usize {
    match active_lease_records(lease_root) {
        Ok(records) => records.len(),
        Err(error) => {
            tracing::warn!(error = %error, "无法可信读取构建任务 lease，按至少一个活动任务处理");
            1
        }
    }
}

pub(crate) fn active_reserved_bytes(lease_root: &Path) -> Result<u64> {
    Ok(active_lease_records(lease_root)?
        .into_iter()
        .fold(0_u64, |total, record| {
            total.saturating_add(record.reserved_bytes)
        }))
}

pub(crate) fn active_lease_records(lease_root: &Path) -> Result<Vec<LeaseRecord>> {
    let root = lease_root.parent().and_then(Path::parent).ok_or_else(|| {
        anyhow!(
            "构建任务 lease 目录不在节点 cache 根内: {}",
            lease_root.display()
        )
    })?;
    match std::fs::symlink_metadata(lease_root) {
        Ok(_) => ensure_existing_within_root(root, lease_root)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法检查构建任务 lease 目录 {}", lease_root.display()));
        }
    }
    let entries = std::fs::read_dir(lease_root)
        .with_context(|| format!("无法枚举构建任务 lease 目录 {}", lease_root.display()))?;
    let mut records = Vec::new();
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("无法读取构建任务 lease 项 {}", lease_root.display())
                });
            }
        };
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            bail!("构建任务 lease 文件名不是有效 UTF-8: {}", path.display());
        };
        if file_name.starts_with('.') && file_name.contains(".lease.json.tmp-") {
            cleanup_abandoned_pending(&path);
            continue;
        }
        if !file_name.ends_with(".lease.json") {
            bail!(
                "构建任务 lease 目录包含未知文件，拒绝清理: {}",
                path.display()
            );
        }
        match ensure_existing_within_root(root, &path) {
            Ok(()) => {}
            Err(error) if !path.exists() => continue,
            Err(error) => return Err(error),
        }
        if lease_is_stale(&path)? {
            match remove_file_if_present(&path) {
                Ok(()) => {}
                Err(error) if !path.exists() => {}
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("无法移除过期构建任务 lease: {}", path.display())
                    });
                }
            }
            continue;
        }
        let payload = match std::fs::read_to_string(&path) {
            Ok(payload) => payload,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("无法读取构建任务 lease: {}", path.display()));
            }
        };
        let mut record = serde_json::from_str::<LeaseRecord>(&payload)
            .with_context(|| format!("构建任务 lease 内容无效: {}", path.display()))?;
        validate_record(&record, &path)?;
        if record.task_key.trim().is_empty() {
            record.task_key = stable_task_key(&record.task_id);
        }
        records.push(record);
    }
    Ok(records)
}

pub(crate) fn runtime_id() -> &'static str {
    static RUNTIME_ID: OnceLock<String> = OnceLock::new();
    RUNTIME_ID
        .get_or_init(|| uuid::Uuid::new_v4().to_string())
        .as_str()
}

fn validate_record(record: &LeaseRecord, path: &Path) -> Result<()> {
    if record.task_id.trim().is_empty()
        || record.project_key.trim().is_empty()
        || record.toolchain_key.trim().is_empty()
    {
        bail!("构建任务 lease 缺少必要字段: {}", path.display());
    }
    Ok(())
}

pub(crate) fn record_task_key(record: &LeaseRecord) -> String {
    if record.task_key.trim().is_empty() {
        stable_task_key(&record.task_id)
    } else {
        record.task_key.clone()
    }
}

fn lease_is_stale(path: &Path) -> Result<bool> {
    let payload = match std::fs::read_to_string(path) {
        Ok(payload) => payload,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("无法读取构建任务 lease: {}", path.display()));
        }
    };
    let record = serde_json::from_str::<LeaseRecord>(&payload)
        .with_context(|| format!("构建任务 lease 内容无效: {}", path.display()))?;
    validate_record(&record, path)?;

    if record.runtime_id == runtime_id() {
        return Ok(!active_path_registry()
            .lock()
            .map_err(|_| anyhow!("构建任务内存 lease 注册表已损坏"))?
            .contains(path));
    }
    if record.process_id > 0 {
        return Ok(!process_is_running(record.process_id));
    }
    Ok(unix_now().saturating_sub(record.created_at_unix_secs) > ACTIVE_LEASE_TTL_SECS)
}

fn cleanup_abandoned_pending(path: &Path) {
    let modified = std::fs::symlink_metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    if unix_now().saturating_sub(modified) > ACTIVE_LEASE_TTL_SECS {
        let _ = remove_file_if_present(path);
    }
}

fn register_active_path(path: &Path) -> Result<()> {
    let inserted = active_path_registry()
        .lock()
        .map_err(|_| anyhow!("构建任务内存 lease 注册表已损坏"))?
        .insert(path.to_path_buf());
    if inserted {
        Ok(())
    } else {
        bail!("构建任务已经在当前节点进程内注册: {}", path.display())
    }
}

fn unregister_active_path(path: &Path) {
    if let Ok(mut active) = active_path_registry().lock() {
        active.remove(path);
    }
}

fn active_path_registry() -> &'static Mutex<HashSet<PathBuf>> {
    static ACTIVE: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}

fn remove_file_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_owned_lease(path: &Path, expected_runtime_id: &str, expected_task_key: &str) {
    let owns_file = std::fs::read_to_string(path)
        .ok()
        .and_then(|payload| serde_json::from_str::<LeaseRecord>(&payload).ok())
        .is_some_and(|record| {
            record.runtime_id == expected_runtime_id
                && record_task_key(&record) == expected_task_key
        });
    if owns_file {
        let _ = remove_file_if_present(path);
    }
}

#[cfg(windows)]
fn process_is_running(process_id: u32) -> bool {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if handle.is_null() {
            return false;
        }
        let mut exit_code = 0_u32;
        let running = GetExitCodeProcess(handle, &mut exit_code) != 0 && exit_code == STILL_ACTIVE;
        CloseHandle(handle);
        running
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
    fn GetExitCodeProcess(process: *mut c_void, exit_code: *mut u32) -> i32;
    fn CloseHandle(object: *mut c_void) -> i32;
}

#[cfg(not(windows))]
fn process_is_running(process_id: u32) -> bool {
    unsafe { kill(process_id as i32, 0) == 0 }
}

#[cfg(not(windows))]
unsafe extern "C" {
    fn kill(process_id: i32, signal: i32) -> i32;
}

#[cfg(test)]
mod tests {
    #[test]
    fn current_process_is_reported_running() {
        assert!(super::process_is_running(std::process::id()));
    }
}
