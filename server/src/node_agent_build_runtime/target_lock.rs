use super::paths::{
    ensure_existing_within_root, ensure_within_root, is_link_or_reparse, BuildRunPaths,
};
use anyhow::{anyhow, bail, Context, Result};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const DEFAULT_TARGET_WAIT_SECS: u64 = 15;
const DEFAULT_ADMISSION_WAIT_SECS: u64 = 30;
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// OS-backed, cross-process exclusive ownership of one project/toolchain
/// target for the complete PreparedBuildRun lifetime.
#[derive(Debug)]
pub(crate) struct TargetLock {
    _file: File,
    path: PathBuf,
}

impl TargetLock {
    pub(crate) fn acquire(paths: &BuildRunPaths, task_id: &str) -> Result<Self> {
        prepare_lock_directory(paths)?;
        let path = paths.target_lock_root.join(format!(
            "{}--{}.lock",
            paths.project_key, paths.toolchain_key
        ));
        let timeout = timeout_from_env(
            "ELON_NODE_TARGET_LOCK_TIMEOUT_SECS",
            DEFAULT_TARGET_WAIT_SECS,
        );
        let mut file = acquire_file(
            &paths.root,
            &path,
            timeout,
            "同项目 Rust target",
        )?;
        write_diagnostic(
            &mut file,
            serde_json::json!({
                "kind": "rust_target",
                "task_id": task_id,
                "task_key": &paths.task_key,
                "project_key": &paths.project_key,
                "toolchain_key": &paths.toolchain_key,
                "process_id": std::process::id(),
                "runtime_id": super::lease::runtime_id(),
                "acquired_at_unix_secs": super::telemetry::unix_now(),
            }),
        )?;
        Ok(Self { _file: file, path })
    }

    #[allow(dead_code)]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// Serializes capacity accounting + lease publication across two accidentally
/// concurrent node-agent processes using the same data root. The in-process
/// prepare mutex alone cannot make reservations atomic across processes.
#[derive(Debug)]
pub(crate) struct AdmissionLock {
    _file: File,
}

impl AdmissionLock {
    pub(crate) fn acquire(paths: &BuildRunPaths, operation: &str) -> Result<Self> {
        Self::acquire_root(&paths.root, operation)
    }

    pub(crate) fn acquire_root(root: &Path, operation: &str) -> Result<Self> {
        let runtime_root = root.join(".runtime");
        let lock_root = runtime_root.join("target-locks");
        std::fs::create_dir_all(&lock_root)
            .with_context(|| format!("无法创建节点 runtime 锁目录 {}", lock_root.display()))?;
        ensure_existing_within_root(root, &lock_root)?;
        let path = runtime_root.join("admission.lock");
        let timeout = timeout_from_env(
            "ELON_NODE_ADMISSION_LOCK_TIMEOUT_SECS",
            DEFAULT_ADMISSION_WAIT_SECS,
        );
        let mut file = acquire_file(root, &path, timeout, "节点构建准入")?;
        write_diagnostic(
            &mut file,
            serde_json::json!({
                "kind": "admission",
                "operation": operation,
                "process_id": std::process::id(),
                "runtime_id": super::lease::runtime_id(),
                "acquired_at_unix_secs": super::telemetry::unix_now(),
            }),
        )?;
        Ok(Self { _file: file })
    }
}

fn prepare_lock_directory(paths: &BuildRunPaths) -> Result<()> {
    std::fs::create_dir_all(&paths.target_lock_root).with_context(|| {
        format!(
            "无法创建节点 runtime 锁目录 {}",
            paths.target_lock_root.display()
        )
    })?;
    ensure_existing_within_root(&paths.root, &paths.target_lock_root)
}

fn acquire_file(
    root: &Path,
    path: &Path,
    timeout: Duration,
    purpose: &str,
) -> Result<File> {
    ensure_within_root(root, path)?;
    let started = Instant::now();
    loop {
        match try_lock(path) {
            Ok(file) => {
                // Do not canonicalize/re-open `path` after a Windows share=0
                // handle is held; that self-conflicts with the exclusive lock.
                // Validate the pinned handle instead.
                validate_locked_file(path, &file)?;
                return Ok(file);
            }
            Err(error) if is_contention(&error) && started.elapsed() < timeout => {
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(error) if is_contention(&error) => {
                return Err(anyhow!(
                    "等待{}独占锁超时（{} 秒）：{}；已有任务或节点进程仍持有该锁，请稍后重试（{}）",
                    purpose,
                    timeout.as_secs(),
                    path.display(),
                    error
                ));
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("无法获取{}独占锁 {}", purpose, path.display()));
            }
        }
    }
}

fn timeout_from_env(name: &str, fallback_secs: u64) -> Duration {
    let secs = std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback_secs);
    Duration::from_secs(secs)
}

fn write_diagnostic(file: &mut File, value: serde_json::Value) -> Result<()> {
    let payload = serde_json::to_vec_pretty(&value)?;
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&payload)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn try_lock(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(windows)]
fn validate_locked_file(path: &Path, file: &File) -> Result<()> {
    let metadata = file
        .metadata()
        .with_context(|| format!("无法检查已锁定文件句柄 {}", path.display()))?;
    if !metadata.is_file() || is_link_or_reparse(&metadata) {
        bail!(
            "节点 runtime 锁必须是普通文件，不能是目录、junction 或重解析点: {}",
            path.display()
        );
    }
    Ok(())
}

#[cfg(windows)]
fn is_contention(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

#[cfg(unix)]
fn try_lock(path: &Path) -> std::io::Result<File> {
    use std::os::fd::AsRawFd;
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    let status = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
    if status == 0 {
        Ok(file)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn validate_locked_file(path: &Path, file: &File) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let path_metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("无法检查已锁定文件路径 {}", path.display()))?;
    let handle_metadata = file
        .metadata()
        .with_context(|| format!("无法检查已锁定文件句柄 {}", path.display()))?;
    if path_metadata.file_type().is_symlink()
        || !handle_metadata.is_file()
        || path_metadata.dev() != handle_metadata.dev()
        || path_metadata.ino() != handle_metadata.ino()
    {
        bail!("节点 runtime 锁路径在打开期间发生替换或指向符号链接: {}", path.display());
    }
    Ok(())
}

#[cfg(unix)]
fn is_contention(error: &std::io::Error) -> bool {
    matches!(error.kind(), std::io::ErrorKind::WouldBlock)
        || matches!(error.raw_os_error(), Some(11 | 35))
}

#[cfg(unix)]
#[link(name = "c")]
unsafe extern "C" {
    fn flock(file_descriptor: i32, operation: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_lock_rejects_a_second_handle() {
        let root = std::env::temp_dir().join(format!(
            "elon-target-lock-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("target.lock");

        let first = try_lock(&path).expect("first lock");
        validate_locked_file(&path, &first).expect("validate first lock");
        let second = try_lock(&path).expect_err("second lock must contend");
        assert!(is_contention(&second));

        drop(first);
        let _ = std::fs::remove_dir_all(root);
    }
}
