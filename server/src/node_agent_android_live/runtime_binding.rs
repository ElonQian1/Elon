use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::adb_session::RuntimeStartEvidence;
use super::broker::{LiveUiBroker, LiveUiSession};
use super::build_verify::wait_for_runtime;
use super::fit_run::workspace_fingerprint;

const SCHEMA_VERSION: u32 = 1;
const MAX_BINDINGS: usize = 64;
static STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DurableRuntimeBinding {
    pub project_root: String,
    pub device_id: String,
    pub package_name: String,
    pub source_revision: String,
    pub root_task_id: String,
    pub updated_at: String,
}

pub(crate) enum RuntimeBindingSelection {
    Bound(DurableRuntimeBinding, bool),
    Control { diagnostic: String },
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindingFile {
    schema_version: u32,
    bindings: Vec<DurableRuntimeBinding>,
}

fn store_path() -> PathBuf {
    crate::state_path().with_file_name("android-live-runtime-bindings.json")
}

fn lock() -> &'static Mutex<()> {
    STORE_LOCK.get_or_init(|| Mutex::new(()))
}

fn read(path: &Path) -> Result<BindingFile> {
    if !path.exists() {
        return Ok(BindingFile {
            schema_version: SCHEMA_VERSION,
            bindings: Vec::new(),
        });
    }
    let bytes =
        fs::read(path).with_context(|| format!("读取 Runtime 绑定失败: {}", path.display()))?;
    if bytes.len() > 256 * 1024 {
        bail!("RUNTIME_BINDING_STORE_INVALID: 持久绑定文件超过大小限制");
    }
    let file: BindingFile = serde_json::from_slice(&bytes)
        .with_context(|| format!("解析 Runtime 绑定失败: {}", path.display()))?;
    if file.schema_version != SCHEMA_VERSION {
        bail!(
            "RUNTIME_BINDING_STORE_INVALID: 不支持 schemaVersion={}",
            file.schema_version
        );
    }
    Ok(file)
}

fn write(path: &Path, file: &BindingFile) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(file)?;
    crate::node_agent_atomic_file::write(path, &bytes)
        .with_context(|| format!("持久化 Runtime 绑定失败: {}", path.display()))
}

pub(crate) fn project_identity(project_root: &str) -> Result<(String, String, String)> {
    let canonical = PathBuf::from(project_root)
        .canonicalize()
        .with_context(|| format!("项目目录不存在: {project_root}"))?;
    let canonical_text = canonical.to_string_lossy().to_string();
    let source_revision = workspace_fingerprint(&canonical_text)?
        .ok_or_else(|| anyhow::anyhow!("RUNTIME_BINDING_STALE: 无法计算项目源码 revision"))?;
    let root_task_id =
        crate::node_agent_supervision_project_identity::resolve_root_task_id(&canonical)?;
    Ok((canonical_text, source_revision, root_task_id))
}

pub(crate) fn persist_verified(session: &LiveUiSession) -> Result<DurableRuntimeBinding> {
    if session.device_id == "ui-design-bootstrap" || session.package_name == "ui.design.bootstrap" {
        bail!("RUNTIME_BINDING_PSEUDO_REJECTED: 伪 Runtime 永不持久化");
    }
    let project_root = session
        .project_root
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("RUNTIME_BINDING_MISSING: session 没有项目目录"))?;
    let (project_root, source_revision, root_task_id) = project_identity(project_root)?;
    let binding = DurableRuntimeBinding {
        project_root,
        device_id: session.device_id.clone(),
        package_name: session.package_name.clone(),
        source_revision,
        root_task_id,
        updated_at: Utc::now().to_rfc3339(),
    };
    let _guard = lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Runtime 绑定锁已损坏"))?;
    let path = store_path();
    let mut file = read(&path)?;
    // A project/root/source tuple has exactly one active Runtime device. Rebinding
    // from an offline phone to an online emulator must replace the old candidate;
    // retaining both would make the next bootstrap fail as ambiguous.
    file.bindings
        .retain(|item| !same_binding_scope(item, &binding));
    file.bindings.push(binding.clone());
    file.bindings
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    file.bindings.truncate(MAX_BINDINGS);
    write(&path, &file)?;
    Ok(binding)
}

fn candidates_for(
    path: &Path,
    project_root: &str,
    source_revision: &str,
    root_task_id: &str,
) -> Result<Vec<DurableRuntimeBinding>> {
    let _guard = lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("Runtime 绑定锁已损坏"))?;
    let all = read(path)?
        .bindings
        .into_iter()
        .filter(|item| same_root(&item.project_root, project_root))
        .collect::<Vec<_>>();
    if all.is_empty() {
        bail!("RUNTIME_BINDING_MISSING: 项目没有持久真实 Runtime 绑定；请在 PC UI 重新连接 com.elon.app.uitest");
    }
    let matching_root = all
        .iter()
        .filter(|item| item.root_task_id == root_task_id)
        .cloned()
        .collect::<Vec<_>>();
    if matching_root.is_empty() {
        bail!(
            "RUNTIME_BINDING_STALE: root task identity 已变化；候选={}",
            candidate_text(&all)
        );
    }
    let fresh = matching_root
        .iter()
        .filter(|item| item.source_revision == source_revision)
        .cloned()
        .collect::<Vec<_>>();
    if fresh.is_empty() {
        bail!(
            "RUNTIME_BINDING_STALE: source revision 已变化；候选={}",
            candidate_text(&matching_root)
        );
    }
    if fresh.len() != 1 {
        bail!(
            "RUNTIME_BINDING_AMBIGUOUS: 找到 {} 个真实设备绑定；候选={}",
            fresh.len(),
            candidate_text(&fresh)
        );
    }
    Ok(fresh)
}

fn candidates_with_legacy_fit_runs(
    path: &Path,
    project_root: &str,
    source_revision: &str,
    root_task_id: &str,
) -> Result<Vec<DurableRuntimeBinding>> {
    match candidates_for(path, project_root, source_revision, root_task_id) {
        Ok(bindings) => Ok(bindings),
        Err(error) if error.to_string().contains("RUNTIME_BINDING_MISSING") => {
            let bindings = super::fit_run::durable_runtime_candidates(project_root)?
                .into_iter()
                // Historical FitRun manifests predate the root-task field. Their
                // persisted task id must still resolve through the durable lineage
                // to this exact root; never stamp an arbitrary current task identity.
                .filter(|(_, _, revision, _, _)| revision.as_deref() == Some(source_revision))
                .filter(|(device, package, _, _, _)| {
                    device != "ui-design-bootstrap" && package != "ui.design.bootstrap"
                })
                .filter(|(_, _, _, task_id, _)| {
                    task_id.as_deref().is_some_and(|task_id| {
                        crate::node_agent_supervision_project_identity::validate_task_root(
                            Path::new(project_root),
                            task_id,
                            root_task_id,
                        )
                        .is_ok()
                    })
                })
                .map(
                    |(device_id, package_name, _, _, updated_at)| DurableRuntimeBinding {
                        project_root: project_root.to_string(),
                        device_id,
                        package_name,
                        source_revision: source_revision.to_string(),
                        root_task_id: root_task_id.to_string(),
                        updated_at,
                    },
                )
                .collect::<Vec<_>>();
            let mut unique = std::collections::BTreeMap::new();
            for binding in bindings {
                unique.insert(
                    (binding.device_id.clone(), binding.package_name.clone()),
                    binding,
                );
            }
            let bindings = unique.into_values().collect::<Vec<_>>();
            match bindings.len() {
                1 => Ok(bindings),
                0 => Err(error),
                count => bail!(
                    "RUNTIME_BINDING_AMBIGUOUS: FitRun 中找到 {count} 个真实设备绑定；候选={}",
                    candidate_text(&bindings)
                ),
            }
        }
        Err(error) => Err(error),
    }
}

fn candidate_text(items: &[DurableRuntimeBinding]) -> String {
    items
        .iter()
        .map(|item| {
            format!(
                "{}:{}@{}",
                item.device_id, item.package_name, item.updated_at
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn same_root(left: &str, right: &str) -> bool {
    left.trim_end_matches(['/', '\\'])
        .eq_ignore_ascii_case(right.trim_end_matches(['/', '\\']))
}

fn same_binding_scope(left: &DurableRuntimeBinding, right: &DurableRuntimeBinding) -> bool {
    same_root(&left.project_root, &right.project_root)
        && left.source_revision == right.source_revision
        && left.root_task_id == right.root_task_id
}

pub(crate) async fn reconnect_or_rebind(
    broker: &LiveUiBroker,
    current: &std::sync::Arc<LiveUiSession>,
    host_port: u16,
) -> Result<(std::sync::Arc<LiveUiSession>, RuntimeStartEvidence, bool)> {
    let devices = crate::node_agent_android_inspector::adb_command::run_adb_text(
        &["devices".into()],
        Duration::from_secs(5),
        64 * 1024,
    )
    .await
    .context("RUNTIME_REBIND_DEVICE_SCAN_FAILED")?;
    let candidates = reconnect_device_candidates(&current.device_id, &devices);
    if candidates.is_empty() {
        bail!(
            "RUNTIME_REBIND_DEVICE_MISSING: 当前绑定 {} 离线，且没有在线 Android 设备",
            current.device_id
        );
    }

    let mut failures = Vec::new();
    for device_id in candidates {
        if !package_is_installed(&device_id, &current.package_name).await {
            failures.push(format!("{device_id}:PACKAGE_MISSING"));
            continue;
        }
        let rebound = device_id != current.device_id;
        let session = if rebound {
            broker
                .create_session_with_identity(
                    device_id.clone(),
                    device_id.clone(),
                    current.package_name.clone(),
                    current.project_root.clone(),
                    current.debug_project_id.clone(),
                    current.device_port,
                )
                .await
        } else {
            current.clone()
        };
        session.reset_for_redeploy().await;
        let start = match super::adb_session::start_runtime(&session, host_port).await {
            Ok(start) => start,
            Err(error) => {
                failures.push(format!("{device_id}:START_FAILED:{error:#}"));
                if rebound {
                    broker.remove_session(&session.id).await;
                }
                continue;
            }
        };
        match tokio::time::timeout(
            Duration::from_secs(20),
            wait_for_runtime(broker, &session.id, &session, None, &start),
        )
        .await
        {
            Ok(Ok(view)) if view.connected && view.node_count > 0 => {
                persist_verified(&session).context(
                    "RUNTIME_REBIND_PERSIST_FAILED: 新 Runtime 已 LIVE，但绑定持久化失败",
                )?;
                return Ok((session, start, rebound));
            }
            Ok(Ok(view)) => failures.push(format!(
                "{device_id}:HANDSHAKE_INCOMPLETE:connected={} nodeCount={}",
                view.connected, view.node_count
            )),
            Ok(Err(error)) => failures.push(format!("{device_id}:HANDSHAKE_FAILED:{error:#}")),
            Err(_) => failures.push(format!("{device_id}:HANDSHAKE_TIMEOUT")),
        }
        if rebound {
            broker.remove_session(&session.id).await;
        }
    }
    bail!(
        "RUNTIME_REBIND_FAILED: 当前绑定={} package={}；候选失败={}",
        current.device_id,
        current.package_name,
        failures.join(" | ")
    )
}

fn reconnect_device_candidates(current_device_id: &str, adb_devices: &str) -> Vec<String> {
    let mut online = adb_devices
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(_, state)| state.split_whitespace().next() == Some("device"))
        .map(|(id, _)| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    online.sort_by_key(|id| {
        (
            id != current_device_id,
            !id.starts_with("emulator-"),
            id.clone(),
        )
    });
    online.dedup();
    online
}

async fn package_is_installed(device_id: &str, package_name: &str) -> bool {
    crate::node_agent_android_inspector::adb_command::run_adb_text(
        &[
            "-s".into(),
            device_id.to_string(),
            "shell".into(),
            "pm".into(),
            "path".into(),
            package_name.to_string(),
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await
    .is_ok_and(|output| {
        output
            .lines()
            .any(|line| line.trim_start().starts_with("package:"))
    })
}

pub(crate) async fn restore_unique(
    broker: &LiveUiBroker,
    project_root: &str,
    host_port: u16,
) -> Result<(std::sync::Arc<LiveUiSession>, DurableRuntimeBinding)> {
    let (project_root, source_revision, root_task_id) = project_identity(project_root)?;
    let mut selected = candidates_with_legacy_fit_runs(
        &store_path(),
        &project_root,
        &source_revision,
        &root_task_id,
    )?;
    let binding = selected.remove(0);
    ensure_bound_device_ready(&binding).await?;
    let session = broker
        .create_session(
            binding.device_id.clone(),
            binding.package_name.clone(),
            Some(project_root),
            super::adb_session::DEFAULT_DEVICE_PORT,
        )
        .await;
    let start = super::adb_session::start_runtime(&session, host_port)
        .await
        .context("RUNTIME_REBOOTSTRAP_START_FAILED: 无法启动已绑定真实 Runtime")?;
    let wait = tokio::time::timeout(
        Duration::from_secs(20),
        wait_for_runtime(broker, &session.id, &session, None, &start),
    )
    .await;
    match wait {
        Ok(Ok(view)) if view.node_count > 0 => Ok((session, binding)),
        Ok(Ok(_)) => {
            broker.remove_session(&session.id).await;
            bail!("RUNTIME_REBOOTSTRAP_EMPTY_TREE: Runtime 未上报节点");
        }
        Ok(Err(error)) => {
            broker.remove_session(&session.id).await;
            Err(error).context("RUNTIME_REBOOTSTRAP_FAILED")
        }
        Err(_) => {
            broker.remove_session(&session.id).await;
            bail!("RUNTIME_REBOOTSTRAP_TIMEOUT: 20 秒内未恢复真实 Runtime");
        }
    }
}

pub(crate) async fn select_or_restore(
    broker: &LiveUiBroker,
    project_root: &str,
    host_port: u16,
) -> Result<(std::sync::Arc<LiveUiSession>, DurableRuntimeBinding, bool)> {
    match broker
        .unique_connected_runtime_for_project(project_root)
        .await
    {
        Ok(session) => {
            let binding = persist_verified(&session)?;
            Ok((session, binding, false))
        }
        Err(error) if error.to_string().contains("项目没有已连接") => {
            let (session, binding) = restore_unique(broker, project_root, host_port).await?;
            let persisted = persist_verified(&session)?;
            debug_assert_eq!(binding.device_id, persisted.device_id);
            Ok((session, persisted, true))
        }
        Err(error) => Err(error),
    }
}

/// Creates a repository control-plane session when no durable Android Runtime
/// exists. Repository-scoped tools stay callable, while `effective_session_id`
/// automatically switches to a later real Runtime for the same project.
pub(crate) async fn select_or_control(
    broker: &LiveUiBroker,
    project_root: &str,
    host_port: u16,
) -> Result<(std::sync::Arc<LiveUiSession>, RuntimeBindingSelection)> {
    match select_or_restore(broker, project_root, host_port).await {
        Ok((session, binding, restored)) => {
            Ok((session, RuntimeBindingSelection::Bound(binding, restored)))
        }
        Err(error) if recoverable_control_diagnostic(&error).is_some() => {
            let diagnostic = recoverable_control_diagnostic(&error).expect("checked above");
            Ok((
                broker
                    .create_session(
                        "ui-design-bootstrap".to_string(),
                        "ui.design.bootstrap".to_string(),
                        Some(project_root.to_string()),
                        super::adb_session::DEFAULT_DEVICE_PORT,
                    )
                    .await,
                RuntimeBindingSelection::Control { diagnostic },
            ))
        }
        Err(error) => Err(error),
    }
}

fn recoverable_control_diagnostic(error: &anyhow::Error) -> Option<String> {
    let message = format!("{error:#}");
    [
        "RUNTIME_BINDING_MISSING",
        "RUNTIME_BINDING_STALE",
        "RUNTIME_BINDING_DEVICE_MISSING",
        "RUNTIME_BINDING_PACKAGE_MISSING",
        "RUNTIME_REBOOTSTRAP_START_FAILED",
        "RUNTIME_REBOOTSTRAP_EMPTY_TREE",
        "RUNTIME_REBOOTSTRAP_FAILED",
        "RUNTIME_REBOOTSTRAP_TIMEOUT",
    ]
    .into_iter()
    .find(|code| message.contains(code))
    .map(str::to_string)
}

pub(crate) fn descriptor_view(project_root: &str, binding: RuntimeBindingSelection) -> Value {
    match binding {
        RuntimeBindingSelection::Bound(binding, restored) => json!({
            "status": "BOUND", "projectRoot": binding.project_root,
            "deviceId": binding.device_id, "packageName": binding.package_name,
            "sourceRevision": binding.source_revision, "rootTaskId": binding.root_task_id,
            "restoredAfterRestart": restored,
        }),
        RuntimeBindingSelection::Control { diagnostic } => json!({
            "status": "UNAVAILABLE", "projectRoot": project_root,
            "diagnostic": diagnostic, "controlPlaneAvailable": true,
            "rebindAvailable": true,
        }),
    }
}

async fn ensure_bound_device_ready(binding: &DurableRuntimeBinding) -> Result<()> {
    let devices = crate::node_agent_android_inspector::adb_command::run_adb_text(
        &["devices".into()],
        Duration::from_secs(5),
        64 * 1024,
    )
    .await
    .context("RUNTIME_BINDING_DEVICE_SCAN_FAILED")?;
    let online = devices
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(_, state)| state.trim() == "device")
        .map(|(id, _)| id.trim().to_string())
        .collect::<Vec<_>>();
    if !online.iter().any(|id| id == &binding.device_id) {
        bail!(
            "RUNTIME_BINDING_DEVICE_MISSING: 绑定设备 {} 不在线；在线候选={}",
            binding.device_id,
            online.join(",")
        );
    }
    let package = crate::node_agent_android_inspector::adb_command::run_adb_text(
        &[
            "-s".into(),
            binding.device_id.clone(),
            "shell".into(),
            "pm".into(),
            "path".into(),
            binding.package_name.clone(),
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await
    .context("RUNTIME_BINDING_PACKAGE_CHECK_FAILED")?;
    if !package
        .lines()
        .any(|line| line.trim_start().starts_with("package:"))
    {
        bail!(
            "RUNTIME_BINDING_PACKAGE_MISSING: 设备 {} 未安装 {}",
            binding.device_id,
            binding.package_name
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(root: &str, device: &str, revision: &str) -> DurableRuntimeBinding {
        DurableRuntimeBinding {
            project_root: root.into(),
            device_id: device.into(),
            package_name: "com.elon.app.uitest".into(),
            source_revision: revision.into(),
            root_task_id: "root-1".into(),
            updated_at: device.into(),
        }
    }

    #[test]
    fn classifies_missing_ambiguous_and_stale_bindings() {
        let dir =
            std::env::temp_dir().join(format!("runtime-binding-{}", uuid::Uuid::new_v4().simple()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bindings.json");
        write(
            &path,
            &BindingFile {
                schema_version: SCHEMA_VERSION,
                bindings: vec![],
            },
        )
        .unwrap();
        assert!(candidates_for(&path, "C:/project", "rev-1", "root-1")
            .unwrap_err()
            .to_string()
            .contains("RUNTIME_BINDING_MISSING"));
        write(
            &path,
            &BindingFile {
                schema_version: SCHEMA_VERSION,
                bindings: vec![binding("C:/project", "device-1", "old")],
            },
        )
        .unwrap();
        assert!(candidates_for(&path, "C:/project", "rev-1", "root-1")
            .unwrap_err()
            .to_string()
            .contains("RUNTIME_BINDING_STALE"));
        write(
            &path,
            &BindingFile {
                schema_version: SCHEMA_VERSION,
                bindings: vec![
                    binding("C:/project", "device-1", "rev-1"),
                    binding("C:/project", "device-2", "rev-1"),
                ],
            },
        )
        .unwrap();
        let error = candidates_for(&path, "C:/project", "rev-1", "root-1")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("RUNTIME_BINDING_AMBIGUOUS")
                && error.contains("device-1")
                && error.contains("device-2")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn selects_one_identity_bound_real_runtime() {
        let dir =
            std::env::temp_dir().join(format!("runtime-binding-{}", uuid::Uuid::new_v4().simple()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bindings.json");
        write(
            &path,
            &BindingFile {
                schema_version: SCHEMA_VERSION,
                bindings: vec![binding("C:/project", "device-1", "rev-1")],
            },
        )
        .unwrap();
        let selected = candidates_for(&path, "c:/PROJECT/", "rev-1", "root-1").unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].device_id, "device-1");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stale_and_offline_bindings_keep_the_project_control_plane_recoverable() {
        for (message, expected) in [
            (
                "RUNTIME_BINDING_STALE: source revision 已变化",
                "RUNTIME_BINDING_STALE",
            ),
            (
                "outer: RUNTIME_BINDING_DEVICE_MISSING: 绑定设备 offline 不在线",
                "RUNTIME_BINDING_DEVICE_MISSING",
            ),
            (
                "RUNTIME_REBOOTSTRAP_TIMEOUT: 20 秒内未恢复真实 Runtime",
                "RUNTIME_REBOOTSTRAP_TIMEOUT",
            ),
        ] {
            let error = anyhow::anyhow!(message);
            assert_eq!(
                recoverable_control_diagnostic(&error).as_deref(),
                Some(expected)
            );
        }
        assert!(
            recoverable_control_diagnostic(&anyhow::anyhow!("RUNTIME_BINDING_AMBIGUOUS")).is_none()
        );
    }

    #[test]
    fn a_verified_rebind_replaces_the_old_device_candidate() {
        let mut file = BindingFile {
            schema_version: SCHEMA_VERSION,
            bindings: vec![
                binding("C:/project", "offline-phone", "rev-1"),
                binding("C:/other", "other-device", "rev-1"),
            ],
        };
        let replacement = binding("c:/PROJECT/", "emulator-5554", "rev-1");
        file.bindings
            .retain(|item| !same_binding_scope(item, &replacement));
        file.bindings.push(replacement);
        assert_eq!(file.bindings.len(), 2);
        assert!(file
            .bindings
            .iter()
            .any(|item| item.device_id == "emulator-5554"));
        assert!(!file
            .bindings
            .iter()
            .any(|item| item.device_id == "offline-phone"));
    }

    #[test]
    fn reconnect_prefers_current_then_online_emulator_for_an_offline_binding() {
        let devices = "List of devices attached\n\
            offline-phone\toffline\n\
            wifi-device\tdevice product:phone\n\
            emulator-5554\tdevice product:sdk\n";
        assert_eq!(
            reconnect_device_candidates("offline-phone", devices),
            ["emulator-5554", "wifi-device"]
        );

        let current_online = "List of devices attached\n\
            emulator-5554\tdevice product:sdk\n\
            wifi-device\tdevice product:phone\n";
        assert_eq!(
            reconnect_device_candidates("wifi-device", current_online),
            ["wifi-device", "emulator-5554"]
        );
    }
}
