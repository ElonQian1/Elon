//! Immutable executable copies used by CLI sidecars across client updates.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::node_agent_cli_sidecar::{CliSidecarRegistry, CliSidecarSessionRecord};

const WORKER_DIR: &str = "cli-workers";
const WORKER_FILE: &str = "elon-cli-worker.exe";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VersionedCliWorker {
    pub(crate) path: PathBuf,
    pub(crate) release: String,
    pub(crate) sha256: String,
}

pub(crate) fn prepare_versioned_worker(
    current_exe: &Path,
    registry_dir: &Path,
) -> Result<VersionedCliWorker> {
    let source_bytes = fs::read(current_exe)
        .with_context(|| format!("读取 CLI sidecar worker 源程序 {}", current_exe.display()))?;
    let sha256 = hex::encode(Sha256::digest(&source_bytes));
    let release = crate::node_agent_release_identity::current();
    let release_dir = format!("{}-{}", safe_release_fragment(&release), &sha256[..16]);
    let worker_dir = worker_root(registry_dir).join(release_dir);
    let worker_path = worker_dir.join(WORKER_FILE);
    fs::create_dir_all(&worker_dir)
        .with_context(|| format!("创建版本化 CLI worker 目录 {}", worker_dir.display()))?;

    if worker_path.exists() {
        verify_worker(&worker_path, &sha256)?;
        return Ok(VersionedCliWorker {
            path: worker_path,
            release,
            sha256,
        });
    }

    let temporary = worker_dir.join(format!(
        ".{WORKER_FILE}.{}.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    fs::copy(current_exe, &temporary).with_context(|| {
        format!(
            "复制版本化 CLI worker {} -> {}",
            current_exe.display(),
            temporary.display()
        )
    })?;
    verify_worker(&temporary, &sha256)?;
    match fs::rename(&temporary, &worker_path) {
        Ok(()) => {}
        Err(error) if worker_path.exists() => {
            let _ = fs::remove_file(&temporary);
            verify_worker(&worker_path, &sha256)
                .with_context(|| format!("并发创建版本化 CLI worker 后校验失败: {error}"))?;
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error)
                .with_context(|| format!("发布版本化 CLI worker {}", worker_path.display()));
        }
    }

    Ok(VersionedCliWorker {
        path: worker_path,
        release,
        sha256,
    })
}

/// Remove only workers whose every recorded task is terminal and whose
/// recorded sidecar process no longer exists. Unknown/orphaned files are kept
/// for manual diagnosis instead of being guessed safe to delete.
pub(crate) fn cleanup_terminal_workers(registry: &CliSidecarRegistry) -> Result<Vec<PathBuf>> {
    let root = worker_root(&registry.dir());
    let sessions = registry.all_sessions()?;
    let mut by_worker: BTreeMap<String, Vec<&CliSidecarSessionRecord>> = BTreeMap::new();
    for session in &sessions {
        if let Some(worker_path) = session
            .worker_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            by_worker
                .entry(worker_path.to_string())
                .or_default()
                .push(session);
        }
    }

    let mut removed = Vec::new();
    for (worker_path, worker_sessions) in by_worker {
        let worker_path = PathBuf::from(worker_path);
        if !is_managed_worker_path(&worker_path, &root)
            || !worker_sessions.iter().all(|session| session.is_terminal())
            || worker_sessions
                .iter()
                .filter_map(|session| session.sidecar_pid)
                .any(process_is_running)
        {
            continue;
        }
        if worker_path.exists() {
            fs::remove_file(&worker_path).with_context(|| {
                format!("清理已终态版本化 CLI worker {}", worker_path.display())
            })?;
            removed.push(worker_path.clone());
        }
        if let Some(parent) = worker_path.parent() {
            let is_empty = fs::read_dir(parent)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false);
            if is_empty {
                let _ = fs::remove_dir(parent);
            }
        }
    }
    Ok(removed)
}

fn worker_root(registry_dir: &Path) -> PathBuf {
    registry_dir
        .parent()
        .map(|parent| parent.join(WORKER_DIR))
        .unwrap_or_else(|| registry_dir.join(WORKER_DIR))
}

fn verify_worker(path: &Path, expected_sha256: &str) -> Result<()> {
    let actual =
        hex::encode(Sha256::digest(fs::read(path).with_context(|| {
            format!("读取版本化 CLI worker {}", path.display())
        })?));
    if actual != expected_sha256 {
        bail!(
            "版本化 CLI worker 不可变校验失败: {} expected {} actual {}",
            path.display(),
            expected_sha256,
            actual
        );
    }
    Ok(())
}

fn safe_release_fragment(value: &str) -> String {
    let fragment: String = value
        .chars()
        .take(64)
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if fragment.is_empty() {
        "unknown".to_string()
    } else {
        fragment
    }
}

fn is_managed_worker_path(worker: &Path, root: &Path) -> bool {
    worker.file_name().and_then(|value| value.to_str()) == Some(WORKER_FILE)
        && worker
            .parent()
            .and_then(Path::parent)
            .is_some_and(|parent| same_path(parent, root))
}

fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

#[cfg(windows)]
pub(crate) fn process_is_running(process_id: u32) -> bool {
    use std::ffi::c_void;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
        fn GetExitCodeProcess(process: *mut c_void, exit_code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if handle.is_null() {
            false
        } else {
            let mut exit_code = 0_u32;
            let running =
                GetExitCodeProcess(handle, &mut exit_code) != 0 && exit_code == STILL_ACTIVE;
            CloseHandle(handle);
            running
        }
    }
}

#[cfg(windows)]
pub(crate) fn process_identity(process_id: u32) -> Option<String> {
    use std::ffi::c_void;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
        if handle.is_null() {
            return None;
        }
        let mut creation = FileTime { low: 0, high: 0 };
        let mut exit = FileTime { low: 0, high: 0 };
        let mut kernel = FileTime { low: 0, high: 0 };
        let mut user = FileTime { low: 0, high: 0 };
        let ok = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) != 0;
        CloseHandle(handle);
        ok.then(|| {
            let created = ((creation.high as u64) << 32) | creation.low as u64;
            format!("windows:{process_id}:{created}")
        })
    }
}

#[cfg(not(windows))]
pub(crate) fn process_is_running(process_id: u32) -> bool {
    process_id == std::process::id() || Path::new(&format!("/proc/{process_id}")).exists()
}

#[cfg(not(windows))]
pub(crate) fn process_identity(process_id: u32) -> Option<String> {
    let stat = fs::read_to_string(format!("/proc/{process_id}/stat")).ok()?;
    // The comm field can contain spaces and parentheses. Everything after its
    // final ')' starts at field 3; process start time is field 22.
    let tail = stat.rsplit_once(')')?.1.trim();
    let start_ticks = tail.split_whitespace().nth(19)?;
    Some(format!("procfs:{process_id}:{start_ticks}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_cli_sidecar::CliSidecarSessionRecord;

    fn fixture() -> (PathBuf, PathBuf, CliSidecarRegistry) {
        let root = std::env::temp_dir().join(format!(
            "elon-versioned-cli-worker-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let source = root.join("installed-client.exe");
        let registry = CliSidecarRegistry::new(root.join("state").join("cli-sidecars"));
        fs::create_dir_all(&root).expect("create worker fixture");
        fs::write(&source, b"worker-release-one").expect("write worker source");
        (root, source, registry)
    }

    fn session(
        task_id: &str,
        worker: &VersionedCliWorker,
        state: &str,
        sidecar_pid: Option<u32>,
    ) -> CliSidecarSessionRecord {
        let mut record = CliSidecarSessionRecord::managed_pipe_json(
            format!("session-{task_id}"),
            task_id,
            "codex",
            "route_a_external_cli",
            None,
            None,
            sidecar_pid,
            None,
            1,
        );
        record.state = state.to_string();
        record.worker_path = Some(worker.path.to_string_lossy().to_string());
        record.worker_release = Some(worker.release.clone());
        record.worker_sha256 = Some(worker.sha256.clone());
        record
    }

    #[test]
    fn repeated_prepare_is_idempotent_and_release_change_keeps_old_worker() {
        let (root, source, registry) = fixture();
        let first = prepare_versioned_worker(&source, &registry.dir()).expect("first worker");
        let duplicate =
            prepare_versioned_worker(&source, &registry.dir()).expect("duplicate worker");
        assert_eq!(first, duplicate);

        fs::write(&source, b"worker-release-two").expect("update source fixture");
        let second = prepare_versioned_worker(&source, &registry.dir()).expect("second worker");
        assert_ne!(first.path, second.path);
        assert!(first.path.exists());
        assert!(second.path.exists());
        fs::remove_dir_all(root).expect("remove worker fixture");
    }

    #[test]
    fn immutable_worker_rejects_corruption_instead_of_overwriting() {
        let (root, source, registry) = fixture();
        let worker = prepare_versioned_worker(&source, &registry.dir()).expect("prepare worker");
        fs::write(&worker.path, b"corrupted").expect("corrupt worker fixture");
        let error = prepare_versioned_worker(&source, &registry.dir())
            .expect_err("corrupted immutable worker must fail closed");
        assert!(error.to_string().contains("不可变校验失败"));
        fs::remove_dir_all(root).expect("remove worker fixture");
    }

    #[test]
    fn cleanup_requires_terminal_task_and_dead_process() {
        let (root, source, registry) = fixture();
        let worker = prepare_versioned_worker(&source, &registry.dir()).expect("prepare worker");
        registry
            .upsert_session(session("running", &worker, "running", None))
            .expect("record running worker");
        assert!(cleanup_terminal_workers(&registry).unwrap().is_empty());

        registry
            .upsert_session(session(
                "running",
                &worker,
                "finished",
                Some(std::process::id()),
            ))
            .expect("record terminal live worker");
        assert!(cleanup_terminal_workers(&registry).unwrap().is_empty());

        registry
            .upsert_session(session("running", &worker, "finished", None))
            .expect("record terminal dead worker");
        assert_eq!(
            cleanup_terminal_workers(&registry).unwrap(),
            vec![worker.path]
        );
        fs::remove_dir_all(root).expect("remove worker fixture");
    }
}
