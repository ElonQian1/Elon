//! Cross-process transaction and crash recovery for `cli-sidecars/sessions.json`.

use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};

use super::CliSidecarSessionRecord;

type Sessions = BTreeMap<String, CliSidecarSessionRecord>;

const LOCK_WAIT: Duration = Duration::from_secs(10);
const LOCK_POLL: Duration = Duration::from_millis(10);

pub(super) fn read<T>(dir: &Path, operation: impl FnOnce(&Sessions) -> Result<T>) -> Result<T> {
    std::fs::create_dir_all(dir).with_context(|| format!("创建 {:?}", dir))?;
    let _lock = RegistryLock::acquire(&dir.join("sessions.lock"))?;
    let sessions = load_or_recover(dir)?;
    operation(&sessions)
}

pub(super) fn update<T>(
    dir: &Path,
    operation: impl FnOnce(&mut Sessions) -> Result<T>,
) -> Result<T> {
    std::fs::create_dir_all(dir).with_context(|| format!("创建 {:?}", dir))?;
    let _lock = RegistryLock::acquire(&dir.join("sessions.lock"))?;
    let mut sessions = load_or_recover(dir)?;
    let result = operation(&mut sessions)?;
    save_with_backup(dir, &sessions)?;
    Ok(result)
}

fn load_or_recover(dir: &Path) -> Result<Sessions> {
    let primary = dir.join("sessions.json");
    let backup = dir.join("sessions.json.bak");
    if !primary.exists() {
        if !backup.exists() {
            return Ok(Sessions::new());
        }
        let (sessions, bytes) = parse_sessions(&backup)?;
        write_atomic(&primary, &bytes)?;
        return Ok(sessions);
    }
    match parse_sessions(&primary) {
        Ok((sessions, _)) => Ok(sessions),
        Err(primary_error) => {
            let (sessions, bytes) = parse_sessions(&backup).with_context(|| {
                format!(
                    "sidecar sessions 主文件损坏且备份不可恢复（主文件错误: {primary_error:#}）"
                )
            })?;
            write_atomic(&primary, &bytes)
                .context("从最近有效备份原子重建 sidecar sessions 主文件")?;
            Ok(sessions)
        }
    }
}

fn parse_sessions(path: &Path) -> Result<(Sessions, Vec<u8>)> {
    let bytes = std::fs::read(path).with_context(|| format!("读取 {:?}", path))?;
    let sessions = serde_json::from_slice(&bytes).with_context(|| format!("解析 {:?}", path))?;
    Ok((sessions, bytes))
}

fn save_with_backup(dir: &Path, sessions: &Sessions) -> Result<()> {
    let primary = dir.join("sessions.json");
    let backup = dir.join("sessions.json.bak");
    if let Ok((_, valid_primary)) = parse_sessions(&primary) {
        write_atomic(&backup, &valid_primary).context("更新 sidecar sessions 有效备份")?;
    }
    let bytes = serde_json::to_vec_pretty(sessions)?;
    write_atomic(&primary, &bytes).context("原子写入 sidecar sessions 主文件")
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("sidecar sessions 文件缺少父目录")?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("sessions.json");
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let attempt = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .with_context(|| format!("创建同目录临时文件 {:?}", temporary))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        atomic_replace(&temporary, path)?;
        sync_parent(parent);
        Ok(())
    })();
    if attempt.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    attempt
}

#[cfg(windows)]
fn atomic_replace(from: &Path, to: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    let ok = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        bail!(std::io::Error::last_os_error());
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
    std::fs::rename(from, to).with_context(|| format!("原子替换 {:?} -> {:?}", from, to))
}

#[cfg(unix)]
fn sync_parent(parent: &Path) {
    let _ = File::open(parent).and_then(|file| file.sync_all());
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) {}

struct RegistryLock {
    _file: File,
}

impl RegistryLock {
    fn acquire(path: &Path) -> Result<Self> {
        let started = Instant::now();
        loop {
            match try_lock(path) {
                Ok(file) => return Ok(Self { _file: file }),
                Err(error) if is_contention(&error) && started.elapsed() < LOCK_WAIT => {
                    std::thread::sleep(LOCK_POLL);
                }
                Err(error) if is_contention(&error) => {
                    bail!(
                        "等待 sidecar sessions 跨进程锁超时: {} ({error})",
                        path.display()
                    )
                }
                Err(error) => return Err(error).with_context(|| format!("获取 {:?}", path)),
            }
        }
    }
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
fn is_contention(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::WouldBlock || matches!(error.raw_os_error(), Some(11 | 35))
}

#[cfg(unix)]
#[link(name = "c")]
unsafe extern "C" {
    fn flock(file_descriptor: i32, operation: i32) -> i32;
}
