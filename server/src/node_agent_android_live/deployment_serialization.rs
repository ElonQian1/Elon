use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, OwnedMutexGuard};

const DEFAULT_CROSS_PROCESS_WAIT_SECS: u64 = 60 * 60;
const CROSS_PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Serializes build/install/handshake for one fixed app on one Android device.
///
/// Different Codex sessions and worktrees on the same PC node intentionally
/// share the node-scoped Debug package. They may edit source concurrently, but
/// only one deployment can own that package's process and Runtime handshake at
/// a time. The in-memory mutex handles one MCP process; the OS-backed file lock
/// closes the same-package race between multiple live MCP sidecars.
#[derive(Default)]
pub(crate) struct DebugDeploymentRegistry {
    node_install_id: Option<String>,
    lock_root: Option<PathBuf>,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

pub(crate) struct DebugDeploymentLease {
    _in_process: OwnedMutexGuard<()>,
    _cross_process: Option<File>,
}

impl DebugDeploymentRegistry {
    pub(crate) fn for_node(install_id: &str, integration_root: PathBuf) -> Self {
        Self {
            node_install_id: Some(install_id.trim().to_string()),
            lock_root: Some(integration_root.join(".deployment-locks")),
            locks: Mutex::default(),
        }
    }

    pub(crate) fn node_install_id(&self) -> Option<&str> {
        self.node_install_id.as_deref()
    }

    pub(crate) async fn acquire(
        &self,
        device_id: &str,
        package_name: &str,
    ) -> Result<DebugDeploymentLease> {
        let key = format!("{}\n{}", device_id.trim(), package_name.trim());
        let lock = self
            .locks
            .lock()
            .await
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let in_process = lock.lock_owned().await;
        let cross_process = match self.lock_root.clone() {
            Some(root) => {
                let device_id = device_id.trim().to_string();
                let package_name = package_name.trim().to_string();
                Some(
                    tokio::task::spawn_blocking(move || {
                        acquire_cross_process_lock(&root, &device_id, &package_name)
                    })
                    .await
                    .context("Android Debug 跨进程部署锁任务异常退出")??,
                )
            }
            None => None,
        };
        Ok(DebugDeploymentLease {
            _in_process: in_process,
            _cross_process: cross_process,
        })
    }
}

fn acquire_cross_process_lock(root: &Path, device_id: &str, package_name: &str) -> Result<File> {
    std::fs::create_dir_all(root)
        .with_context(|| format!("无法创建 Android Debug 部署锁目录 {}", root.display()))?;
    let digest = Sha256::digest(format!("{device_id}\n{package_name}").as_bytes());
    let path = root.join(format!("{}.lock", hex::encode(&digest[..16])));
    let timeout = cross_process_wait_timeout();
    let started = Instant::now();
    loop {
        match try_lock(&path) {
            Ok(mut file) => {
                validate_locked_file(&path, &file)?;
                let diagnostic = serde_json::to_vec_pretty(&serde_json::json!({
                    "kind": "android_debug_deployment",
                    "deviceId": device_id,
                    "packageName": package_name,
                    "processId": std::process::id(),
                    "acquiredAt": chrono::Utc::now().to_rfc3339(),
                }))?;
                file.set_len(0)?;
                file.seek(SeekFrom::Start(0))?;
                file.write_all(&diagnostic)?;
                file.write_all(b"\n")?;
                file.sync_all()?;
                return Ok(file);
            }
            Err(error) if is_contention(&error) && started.elapsed() < timeout => {
                std::thread::sleep(CROSS_PROCESS_POLL_INTERVAL);
            }
            Err(error) if is_contention(&error) => {
                return Err(anyhow!(
                    "等待 Android Debug 独占部署锁超时（{} 秒）：device={} package={} path={}；已有 MCP 进程仍在构建、安装或恢复 Runtime",
                    timeout.as_secs(),
                    device_id,
                    package_name,
                    path.display()
                ));
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("无法获取 Android Debug 跨进程部署锁 {}", path.display())
                });
            }
        }
    }
}

fn cross_process_wait_timeout() -> Duration {
    let seconds = std::env::var("ELON_ANDROID_DEBUG_DEPLOYMENT_LOCK_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_CROSS_PROCESS_WAIT_SECS);
    Duration::from_secs(seconds)
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
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    let metadata = file
        .metadata()
        .with_context(|| format!("无法检查 Android Debug 锁文件 {}", path.display()))?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        anyhow::bail!(
            "Android Debug 部署锁必须是普通文件，不能是目录或重解析点: {}",
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
        .with_context(|| format!("无法检查 Android Debug 锁路径 {}", path.display()))?;
    let handle_metadata = file
        .metadata()
        .with_context(|| format!("无法检查 Android Debug 锁句柄 {}", path.display()))?;
    if path_metadata.file_type().is_symlink()
        || !handle_metadata.is_file()
        || path_metadata.dev() != handle_metadata.dev()
        || path_metadata.ino() != handle_metadata.ino()
    {
        anyhow::bail!(
            "Android Debug 部署锁路径在打开期间发生替换或指向符号链接: {}",
            path.display()
        );
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
    use std::path::PathBuf;
    use std::time::Duration;

    use super::DebugDeploymentRegistry;

    #[tokio::test]
    async fn same_device_and_package_wait_for_the_active_deployment() {
        let registry = DebugDeploymentRegistry::default();
        let first = registry
            .acquire("phone-a", "com.elon.app.debug")
            .await
            .unwrap();
        assert!(tokio::time::timeout(
            Duration::from_millis(20),
            registry.acquire("phone-a", "com.elon.app.debug")
        )
        .await
        .is_err());
        drop(first);
        tokio::time::timeout(
            Duration::from_millis(100),
            registry.acquire("phone-a", "com.elon.app.debug"),
        )
        .await
        .expect("deployment slot should become available")
        .unwrap();
    }

    #[tokio::test]
    async fn different_node_packages_do_not_block_each_other() {
        let registry = DebugDeploymentRegistry::default();
        let _first = registry
            .acquire("phone-a", "com.elon.app.node_a")
            .await
            .unwrap();
        tokio::time::timeout(
            Duration::from_millis(100),
            registry.acquire("phone-a", "com.elon.app.node_b"),
        )
        .await
        .expect("different node-scoped packages may deploy independently")
        .unwrap();
    }

    #[tokio::test]
    async fn separate_registries_share_the_os_backed_deployment_lock() {
        let root = std::env::temp_dir().join(format!(
            "elon-android-deployment-lock-{}",
            uuid::Uuid::new_v4()
        ));
        let first_registry = DebugDeploymentRegistry::for_node("node-a", PathBuf::from(&root));
        let second_registry = DebugDeploymentRegistry::for_node("node-a", PathBuf::from(&root));
        let first = first_registry
            .acquire("phone-a", "com.elon.app.debug")
            .await
            .unwrap();
        let waiter = tokio::spawn(async move {
            second_registry
                .acquire("phone-a", "com.elon.app.debug")
                .await
        });
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(!waiter.is_finished());
        drop(first);
        let second = tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("cross-process lock should release")
            .expect("waiter task should not panic")
            .expect("second registry should acquire after release");
        drop(second);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn node_identity_is_available_to_all_debug_entry_points() {
        let registry = DebugDeploymentRegistry::for_node(" install-a ", std::env::temp_dir());
        assert_eq!(registry.node_install_id(), Some("install-a"));
    }
}
