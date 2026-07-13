use super::{paths::BuildRunPaths, telemetry::unix_now};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) const ACTIVE_LEASE_TTL_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LeaseRecord {
    pub(crate) task_id: String,
    pub(crate) project_key: String,
    pub(crate) toolchain_key: String,
    pub(crate) temp_path: String,
    pub(crate) created_at_unix_secs: u64,
}

#[derive(Debug)]
pub(crate) struct BuildRunLease {
    path: PathBuf,
}

impl BuildRunLease {
    pub(crate) fn acquire(paths: &BuildRunPaths, task_id: &str) -> Result<Self> {
        std::fs::create_dir_all(&paths.lease_root)?;
        let lease_name = format!(
            "{}.lease.json",
            elon_pc_dev_runtime::safe_path_part(task_id, "task", 96)
        );
        let path = paths.lease_root.join(lease_name);
        if path.exists() && lease_is_stale(&path) {
            let _ = std::fs::remove_file(&path);
        }
        let record = LeaseRecord {
            task_id: task_id.to_string(),
            project_key: paths.project_key.clone(),
            toolchain_key: paths.toolchain_key.clone(),
            temp_path: paths.task_temp.to_string_lossy().to_string(),
            created_at_unix_secs: unix_now(),
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("构建任务 lease 已存在或无法创建: {}", path.display()))?;
        serde_json::to_writer_pretty(&mut file, &record)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(Self { path })
    }
}

impl Drop for BuildRunLease {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub(crate) fn active_lease_count(lease_root: &Path) -> usize {
    active_lease_records(lease_root).len()
}

pub(crate) fn active_lease_records(lease_root: &Path) -> Vec<LeaseRecord> {
    let Ok(entries) = std::fs::read_dir(lease_root) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if lease_is_stale(&path) {
                let _ = std::fs::remove_file(path);
                return None;
            }
            std::fs::read_to_string(path)
                .ok()
                .and_then(|payload| serde_json::from_str::<LeaseRecord>(&payload).ok())
        })
        .collect()
}

fn lease_is_stale(path: &Path) -> bool {
    let modified = std::fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    unix_now().saturating_sub(modified) > ACTIVE_LEASE_TTL_SECS
}
