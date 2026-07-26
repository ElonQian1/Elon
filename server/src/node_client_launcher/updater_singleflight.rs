//! Cross-process ownership for Windows automatic update work.
//!
//! The foreground launcher never waits for this owner. File handles are used
//! instead of PID files as authority so an abnormal exit releases ownership
//! without terminating an unrelated process that later reused the same PID.

use super::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEFAULT_BACKGROUND_RETRY_SECS: u64 = 5 * 60;
const MIN_BACKGROUND_RETRY_SECS: u64 = 30;
const MAX_BACKGROUND_RETRY_SECS: u64 = 60 * 60;
const OWNER_HANDOFF_WAIT: Duration = Duration::from_millis(500);

pub(crate) struct UpdateProcessLock {
    _file: File,
}

#[derive(Debug, Deserialize, Serialize)]
struct BackgroundUpdateState {
    schema: String,
    owner_pid: u32,
    started_at_ms: u128,
    finished_at_ms: Option<u128>,
    retry_after_ms: u128,
    outcome: String,
}

pub(crate) fn ensure_background_update(install_dir: &Path) {
    match try_schedule_background_update(install_dir) {
        Ok(true) => {}
        Ok(false) => log_file::record_event(
            install_dir,
            "background_update_reused",
            true,
            "an update owner is already running or the durable retry window is still active",
        ),
        Err(error) => log_file::record_event(
            install_dir,
            "background_update_schedule_failed",
            false,
            &format!("{error:#}"),
        ),
    }
}

pub(crate) fn run_update_owner(install_dir: &Path) -> Result<bool> {
    let _owner = match acquire_owner_lock(install_dir) {
        Ok(lock) => lock,
        Err(error) => {
            log_file::record_event(
                install_dir,
                "background_update_duplicate_skipped",
                true,
                &format!("another update owner holds the process lock: {error}"),
            );
            return Ok(false);
        }
    };

    let started_at_ms = now_ms();
    write_state(
        install_dir,
        BackgroundUpdateState {
            schema: "elon.windows_update_owner.v1".to_string(),
            owner_pid: std::process::id(),
            started_at_ms,
            finished_at_ms: None,
            retry_after_ms: 0,
            outcome: "running".to_string(),
        },
    )?;
    log_file::record_event(
        install_dir,
        "background_update_owner_started",
        true,
        &format!("pid={}; single_flight=file_lock", std::process::id()),
    );

    let result = update_client_if_needed(install_dir);
    let finished_at_ms = now_ms();
    let retry_after_ms =
        finished_at_ms.saturating_add(Duration::from_secs(background_retry_secs()).as_millis());
    let outcome = match &result {
        Ok(true) => "apply_scheduled",
        Ok(false) => "idle_or_deferred",
        Err(_) => "failed",
    };
    if let Err(error) = write_state(
        install_dir,
        BackgroundUpdateState {
            schema: "elon.windows_update_owner.v1".to_string(),
            owner_pid: std::process::id(),
            started_at_ms,
            finished_at_ms: Some(finished_at_ms),
            retry_after_ms,
            outcome: outcome.to_string(),
        },
    ) {
        log_file::record_event(
            install_dir,
            "background_update_state_write_failed",
            false,
            &format!("{error:#}"),
        );
    }
    result
}

pub(crate) fn try_acquire_apply_lock(install_dir: &Path) -> std::io::Result<UpdateProcessLock> {
    acquire_named_lock(install_dir, "update.apply.lock")
}

fn try_schedule_background_update(install_dir: &Path) -> Result<bool> {
    let _spawn_gate = match acquire_named_lock(install_dir, "update.spawn.lock") {
        Ok(lock) => lock,
        Err(_) => return Ok(false),
    };
    if !background_retry_due(install_dir, now_ms()) {
        return Ok(false);
    }
    match acquire_owner_lock(install_dir) {
        Ok(probe) => drop(probe),
        Err(_) => return Ok(false),
    }

    let client = paths::client_exe(install_dir);
    anyhow::ensure!(client.exists(), "缺少后台更新入口：{}", client.display());
    let mut command = launcher_command::silent_command(&client);
    command
        .arg(super::super::BACKGROUND_UPDATE_ARG)
        .current_dir(install_dir)
        .env("NODE_AUTO_OPEN_ADMIN", "0");
    let child = launcher_command::spawn_hidden(&mut command)
        .with_context(|| format!("无法启动后台更新 owner {}", client.display()))?;
    log_file::record_event(
        install_dir,
        "background_update_owner_spawned",
        true,
        &format!(
            "pid={}; arg={}",
            child.id(),
            super::super::BACKGROUND_UPDATE_ARG
        ),
    );

    let deadline = std::time::Instant::now() + OWNER_HANDOFF_WAIT;
    while std::time::Instant::now() < deadline {
        if acquire_owner_lock(install_dir).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(true)
}

fn background_retry_due(install_dir: &Path, now: u128) -> bool {
    let Ok(bytes) = std::fs::read(background_state_path(install_dir)) else {
        return true;
    };
    serde_json::from_slice::<BackgroundUpdateState>(&bytes)
        .map(|state| state.finished_at_ms.is_none() || now >= state.retry_after_ms)
        .unwrap_or(true)
}

fn write_state(install_dir: &Path, state: BackgroundUpdateState) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(&state).context("序列化后台更新 owner 状态")?;
    crate::node_agent_atomic_file::write(&background_state_path(install_dir), &bytes)
}

fn background_state_path(install_dir: &Path) -> PathBuf {
    paths::internal_dir(install_dir).join("update-background-state.json")
}

fn acquire_owner_lock(install_dir: &Path) -> std::io::Result<UpdateProcessLock> {
    acquire_named_lock(install_dir, "update.owner.lock")
}

fn acquire_named_lock(install_dir: &Path, file_name: &str) -> std::io::Result<UpdateProcessLock> {
    use std::os::windows::fs::OpenOptionsExt;

    let internal = paths::internal_dir(install_dir);
    std::fs::create_dir_all(&internal)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .share_mode(0)
        .open(internal.join(file_name))?;
    Ok(UpdateProcessLock { _file: file })
}

fn background_retry_secs() -> u64 {
    env_u64(
        "NODE_AGENT_UPDATE_BACKGROUND_RETRY_SECS",
        DEFAULT_BACKGROUND_RETRY_SECS,
        MIN_BACKGROUND_RETRY_SECS,
        MAX_BACKGROUND_RETRY_SECS,
    )
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-update-singleflight-{label}-{}",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn owner_and_apply_locks_are_single_flight_and_crash_recoverable() {
        let root = root("locks");
        let owner = acquire_owner_lock(&root).expect("first owner");
        assert!(acquire_owner_lock(&root).is_err());
        drop(owner);
        acquire_owner_lock(&root).expect("owner lock recovers after process handle closes");

        let apply = try_acquire_apply_lock(&root).expect("first apply owner");
        assert!(try_acquire_apply_lock(&root).is_err());
        drop(apply);
        try_acquire_apply_lock(&root).expect("apply lock recovers after abnormal owner exit");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn durable_retry_window_coalesces_repeated_launcher_clicks() {
        let root = root("retry");
        write_state(
            &root,
            BackgroundUpdateState {
                schema: "elon.windows_update_owner.v1".to_string(),
                owner_pid: 42,
                started_at_ms: 10,
                finished_at_ms: Some(20),
                retry_after_ms: 5_000,
                outcome: "idle_or_deferred".to_string(),
            },
        )
        .unwrap();

        assert!(!background_retry_due(&root, 4_999));
        assert!(background_retry_due(&root, 5_000));
        assert!(std::fs::read_to_string(background_state_path(&root))
            .unwrap()
            .contains("idle_or_deferred"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn corrupt_scheduler_state_fails_open_without_becoming_authority() {
        let root = root("corrupt");
        std::fs::create_dir_all(paths::internal_dir(&root)).unwrap();
        std::fs::write(background_state_path(&root), b"{broken").unwrap();

        assert!(background_retry_due(&root, 1));
        let _ = std::fs::remove_dir_all(root);
    }
}
