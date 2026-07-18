//! Atomic, cross-process admission for one-time supervision lease migration.

use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};

const ADMISSION_WAIT: Duration = Duration::from_secs(10);
const ADMISSION_POLL: Duration = Duration::from_millis(20);
const REPLACE_ATTEMPTS: usize = 6;

/// Held from authoritative Resume validation until the inherited workspace is
/// registered as active. This serializes two node-agent processes sharing one
/// repository, not just two requests in the same Tokio runtime.
#[derive(Debug)]
pub(crate) struct ResumeAdmissionGuard {
    _file: File,
}

impl ResumeAdmissionGuard {
    pub(crate) fn acquire(base: &Path) -> Result<Self> {
        let common = git_common_dir(base)?;
        let path = common.join("elon-supervision-resume-admission.lock");
        let started = Instant::now();
        loop {
            match try_lock(&path) {
                Ok(file) => {
                    validate_locked_file(&path, &file)?;
                    return Ok(Self { _file: file });
                }
                Err(error) if is_contention(&error) && started.elapsed() < ADMISSION_WAIT => {
                    std::thread::sleep(ADMISSION_POLL);
                }
                Err(error) if is_contention(&error) => {
                    bail!(
                        "等待监督 Resume 跨进程准入锁超时：{} ({error})",
                        path.display()
                    )
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("获取监督 Resume 准入锁 {}", path.display()))
                }
            }
        }
    }
}

/// Replaces the persistent Git worktree lock reason without ever deleting the
/// `locked` file. The caller must have already proved the descendant lineage;
/// this layer independently requires the exact old lease and is one-shot.
pub(crate) fn migrate_legacy_child_lease(
    _admission: &ResumeAdmissionGuard,
    base: &Path,
    active: &Path,
    legacy_task_id: &str,
    root_task_id: &str,
) -> Result<()> {
    anyhow::ensure!(
        legacy_task_id.trim() != root_task_id.trim(),
        "legacy supervision lease already names the root task"
    );
    let previous = super::lease_reason(legacy_task_id)?;
    let expected = super::lease_reason(root_task_id)?;
    let actual = super::worktree_lock_reason(base, active)?;
    anyhow::ensure!(
        actual.as_deref() == Some(previous.as_str()),
        "refusing supervision lease migration: expected legacy {previous}, actual {}",
        actual.as_deref().unwrap_or("<unlocked>")
    );

    let locked = verified_locked_path(base, active)?;
    let installed = std::fs::read_to_string(&locked)
        .with_context(|| format!("读取 Git worktree lease {}", locked.display()))?;
    anyhow::ensure!(
        installed.trim_end_matches(['\r', '\n']) == previous,
        "Git worktree lease file changed before migration"
    );
    replace_locked_file(&locked, format!("{expected}\n").as_bytes())?;
    anyhow::ensure!(
        super::worktree_lock_reason(base, active)?.as_deref() == Some(expected.as_str()),
        "migrated supervision root lease was not persisted"
    );
    Ok(())
}

fn verified_locked_path(base: &Path, active: &Path) -> Result<PathBuf> {
    let common = git_common_dir(base)?;
    let active_common = git_common_dir(active)?;
    anyhow::ensure!(
        same_path(&common, &active_common),
        "worktree Git common-dir drifted"
    );

    let raw = super::git_output(active, &["rev-parse", "--absolute-git-dir"])?;
    let admin = canonical_directory(Path::new(raw.trim()), "worktree Git admin directory")?;
    let worktrees = canonical_directory(&common.join("worktrees"), "Git worktrees directory")?;
    anyhow::ensure!(
        admin
            .parent()
            .is_some_and(|parent| same_path(parent, &worktrees)),
        "worktree Git admin directory is outside the trusted common-dir"
    );
    let locked = admin.join("locked");
    let metadata = std::fs::symlink_metadata(&locked)
        .with_context(|| format!("检查 Git worktree lease {}", locked.display()))?;
    anyhow::ensure!(
        metadata.file_type().is_file(),
        "Git worktree lease is not a regular file"
    );
    Ok(locked)
}

fn git_common_dir(cwd: &Path) -> Result<PathBuf> {
    let raw = super::git_output(
        cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    canonical_directory(Path::new(raw.trim()), "Git common directory")
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf> {
    let path =
        std::fs::canonicalize(path).with_context(|| format!("解析{label} {}", path.display()))?;
    anyhow::ensure!(path.is_dir(), "{label}不是目录: {}", path.display());
    Ok(path)
}

fn replace_locked_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Git worktree lease 缺少父目录")?;
    let temporary = parent.join(format!(
        ".locked.elon-migrate.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .with_context(|| format!("创建 lease 迁移临时文件 {}", temporary.display()))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        let mut last_error = None;
        for attempt in 1..=REPLACE_ATTEMPTS {
            match atomic_replace(&temporary, path) {
                Ok(()) => {
                    sync_parent(parent);
                    return Ok(());
                }
                Err(error) if retryable_replace_error(&error) && attempt < REPLACE_ATTEMPTS => {
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(25 * (1 << (attempt - 1).min(4))));
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow!("lease 原子替换没有产生底层错误")))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.with_context(|| format!("原子迁移 Git worktree lease {}", path.display()))
}

fn retryable_replace_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.downcast_ref::<std::io::Error>().is_some_and(|error| {
            matches!(
                error.kind(),
                std::io::ErrorKind::Interrupted
                    | std::io::ErrorKind::PermissionDenied
                    | std::io::ErrorKind::WouldBlock
            ) || matches!(error.raw_os_error(), Some(5 | 11 | 16 | 32 | 33))
        })
    })
}

#[cfg(windows)]
fn try_lock(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .share_mode(0)
        .open(path)
}

#[cfg(windows)]
fn validate_locked_file(path: &Path, file: &File) -> Result<()> {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    let metadata = file
        .metadata()
        .with_context(|| format!("检查监督 Resume 锁句柄 {}", path.display()))?;
    anyhow::ensure!(
        metadata.is_file() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0,
        "监督 Resume 锁必须是普通文件: {}",
        path.display()
    );
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
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
        Ok(file)
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn validate_locked_file(path: &Path, file: &File) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let path_metadata = std::fs::symlink_metadata(path)?;
    let handle_metadata = file.metadata()?;
    anyhow::ensure!(
        !path_metadata.file_type().is_symlink()
            && handle_metadata.is_file()
            && path_metadata.dev() == handle_metadata.dev()
            && path_metadata.ino() == handle_metadata.ino(),
        "监督 Resume 锁路径发生替换: {}",
        path.display()
    );
    Ok(())
}

#[cfg(unix)]
fn is_contention(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock || matches!(error.raw_os_error(), Some(11 | 35))
}

#[cfg(unix)]
#[link(name = "c")]
unsafe extern "C" {
    fn flock(file_descriptor: i32, operation: i32) -> i32;
}

#[cfg(windows)]
fn atomic_replace(from: &Path, to: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let from_wide = from
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let to_wide = to
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    if unsafe {
        MoveFileExW(
            from_wide.as_ptr(),
            to_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error()).with_context(|| {
            format!(
                "MoveFileExW lease 原子替换失败: {} -> {}",
                from.display(),
                to.display()
            )
        });
    }
    Ok(())
}

#[cfg(windows)]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
}

#[cfg(not(windows))]
fn atomic_replace(from: &Path, to: &Path) -> Result<()> {
    std::fs::rename(from, to)
        .with_context(|| format!("原子替换 {} -> {}", from.display(), to.display()))
}

#[cfg(unix)]
fn sync_parent(parent: &Path) {
    let _ = File::open(parent).and_then(|file| file.sync_all());
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) {}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_command_error::git_command;

    #[test]
    fn resume_admission_guard_is_os_exclusive() {
        let root = std::env::temp_dir().join(format!(
            "elon-resume-admission-lock-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = git_command()
            .arg("init")
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(output.status.success());

        let first = ResumeAdmissionGuard::acquire(&root).unwrap();
        let lock_path = git_common_dir(&root)
            .unwrap()
            .join("elon-supervision-resume-admission.lock");
        let second = try_lock(&lock_path).expect_err("the OS lock must reject another handle");
        assert!(is_contention(&second), "unexpected lock error: {second}");

        drop(first);
        let _ = std::fs::remove_dir_all(root);
    }
}
