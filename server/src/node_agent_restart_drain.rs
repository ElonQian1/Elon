use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::NodeRuntime;

mod admission;
mod classification;
mod startup;
use admission::admit_running_update;
#[cfg(test)]
use admission::{exact_target_identity as admission_exact_target, update_request_matches};
use classification::classify_supervised_tasks;
use classification::StaleCancelProof;
#[cfg(test)]
use classification::{
    drain_task_disposition, load_drain_candidates, startup_checkpoint_disposition,
    DrainTaskDisposition, StartupCheckpointDisposition,
};
use startup::spawn_startup_checkpoint_reconciler;

const CHECKPOINT_PROTOCOL: &str = "elon.local_supervision_restart.v1";
const DRAIN_POLL_SECS: u64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RestartCheckpoint {
    protocol: String,
    update_id: String,
    source: String,
    state: String,
    created_at_ms: u128,
    updated_at_ms: u128,
    active_task_ids: Vec<String>,
    download_url: Option<String>,
    #[serde(default)]
    target_release_identity: Option<String>,
    #[serde(default)]
    recoverable_task_ids: Vec<String>,
    #[serde(default)]
    stale_registry_task_ids: Vec<String>,
    #[serde(default)]
    stale_cancel_proofs: Vec<StaleCancelProof>,
    #[serde(default)]
    superseded_update_id: Option<String>,
    message: String,
}

impl RestartCheckpoint {
    fn draining(
        source: &str,
        task_ids: Vec<String>,
        download_url: Option<String>,
        target_release_identity: Option<String>,
    ) -> Self {
        let now = now_ms();
        Self {
            protocol: CHECKPOINT_PROTOCOL.to_string(),
            update_id: format!("node-update-{}", uuid::Uuid::new_v4().simple()),
            source: source.to_string(),
            state: "draining".to_string(),
            created_at_ms: now,
            updated_at_ms: now,
            active_task_ids: task_ids,
            download_url,
            target_release_identity,
            recoverable_task_ids: Vec::new(),
            stale_registry_task_ids: Vec::new(),
            stale_cancel_proofs: Vec::new(),
            superseded_update_id: None,
            message: "检测到活跃桌面监督任务；更新已排空，安全终态后自动继续。".to_string(),
        }
    }

    fn transition(&mut self, state: &str, message: impl Into<String>) {
        self.state = state.to_string();
        self.message = message.into();
        self.updated_at_ms = now_ms();
    }

    fn payload(&self) -> Value {
        let resume_actions = if self.state == "resume_required" {
            self.active_task_ids
                .iter()
                .map(|task_id| {
                    json!({
                        "task_id": task_id,
                        "action": "Resume",
                        "command": format!("invoke-supervised-task.ps1 -Action Resume -TaskId '{task_id}'"),
                    })
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        json!({
            "protocol": self.protocol,
            "update_id": self.update_id,
            "source": self.source,
            "state": self.state,
            "created_at_ms": self.created_at_ms,
            "updated_at_ms": self.updated_at_ms,
            "active_task_ids": self.active_task_ids,
            "recoverable_task_ids": self.recoverable_task_ids,
            "stale_registry_task_ids": self.stale_registry_task_ids,
            "stale_cancel_proofs": self.stale_cancel_proofs,
            "superseded_update_id": self.superseded_update_id,
            "target_release_identity": self.target_release_identity,
            "message": self.message,
            "resume_actions": resume_actions,
        })
    }
}

pub(crate) async fn schedule_update(
    runtime: Arc<NodeRuntime>,
    source: &str,
    download_url: Option<String>,
    target_release_identity: Option<String>,
) -> Result<Value, String> {
    let classification =
        classify_supervised_tasks(runtime.as_ref(), target_release_identity.as_deref())
            .await
            .map_err(|error| format!("读取监督任务排空状态失败，已拒绝更新：{error:#}"))?;
    let cloud_http_url = runtime.cloud_http_url();
    let admission = drain_admission_lock()
        .lock()
        .map_err(|_| "更新排空准入锁已损坏，已拒绝更新。".to_string())?;
    if drain_running()
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return admit_running_update(
            source,
            download_url.as_deref(),
            target_release_identity.as_deref(),
            &classification,
        );
    }
    let active = classification.blocking;
    if active.is_empty() {
        let mut checkpoint = RestartCheckpoint::draining(
            source,
            Vec::new(),
            download_url.clone(),
            target_release_identity,
        );
        checkpoint.recoverable_task_ids = classification.recoverable;
        checkpoint.stale_registry_task_ids = classification.stale;
        checkpoint.stale_cancel_proofs = classification.stale_cancel_proofs;
        checkpoint.transition("applying", "没有活跃监督任务，正在安全应用更新。");
        if let Err(error) = save_checkpoint(&checkpoint) {
            drain_running().store(false, Ordering::Release);
            return Err(error);
        }
        drop(admission);
        let result = apply_update(&cloud_http_url, download_url.as_deref(), checkpoint).await;
        if result.is_err() {
            drain_running().store(false, Ordering::Release);
        }
        return result;
    }
    let mut checkpoint =
        RestartCheckpoint::draining(source, active, download_url, target_release_identity);
    checkpoint.recoverable_task_ids = classification.recoverable;
    checkpoint.stale_registry_task_ids = classification.stale;
    checkpoint.stale_cancel_proofs = classification.stale_cancel_proofs;
    if let Err(error) = save_checkpoint(&checkpoint) {
        drain_running().store(false, Ordering::Release);
        return Err(error);
    }
    drop(admission);

    let response_checkpoint = checkpoint.clone();
    let runtime_for_drain = runtime.clone();
    let cloud_http_for_drain = cloud_http_url.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(DRAIN_POLL_SECS)).await;
            let observed = {
                let Ok(_admission) = drain_admission_lock().lock() else {
                    continue;
                };
                match load_checkpoint() {
                    Ok(Some(current)) => current,
                    _ => continue,
                }
            };
            let observed_update_id = observed.update_id.clone();
            let classification = match classify_supervised_tasks(
                runtime_for_drain.as_ref(),
                observed.target_release_identity.as_deref(),
            )
            .await
            {
                Ok(classification) => classification,
                Err(error) => {
                    let Ok(_admission) = drain_admission_lock().lock() else {
                        continue;
                    };
                    let Ok(Some(current)) = load_checkpoint() else {
                        continue;
                    };
                    if current.update_id != observed_update_id {
                        continue;
                    }
                    let mut checkpoint = current;
                    checkpoint.message =
                        format!("读取监督任务排空状态失败，更新继续保持延期：{error:#}");
                    checkpoint.updated_at_ms = now_ms();
                    let _ = save_checkpoint(&checkpoint);
                    continue;
                }
            };
            let Ok(_admission) = drain_admission_lock().lock() else {
                continue;
            };
            let Ok(Some(current)) = load_checkpoint() else {
                continue;
            };
            if current.update_id != observed_update_id {
                continue;
            }
            let mut checkpoint = current;
            let active = classification.blocking;
            checkpoint.recoverable_task_ids = classification.recoverable;
            checkpoint.stale_registry_task_ids = classification.stale;
            checkpoint.stale_cancel_proofs = classification.stale_cancel_proofs;
            if !active.is_empty() {
                checkpoint.active_task_ids = active;
                checkpoint.updated_at_ms = now_ms();
                let _ = save_checkpoint(&checkpoint);
                continue;
            }
            checkpoint.active_task_ids.clear();
            checkpoint.transition("applying", "监督任务已安全结束，正在应用延期更新。");
            let _ = save_checkpoint(&checkpoint);
            drop(_admission);
            if let Err(error) = apply_update(
                &cloud_http_for_drain,
                checkpoint.download_url.as_deref(),
                checkpoint.clone(),
            )
            .await
            {
                checkpoint.transition("failed", error);
                let _ = save_checkpoint(&checkpoint);
                drain_running().store(false, Ordering::Release);
            }
            break;
        }
    });

    Ok(json!({
        "ok": true,
        "deferred": true,
        "restart_recovery": response_checkpoint.payload(),
    }))
}

pub(crate) fn recover_checkpoint_after_startup(runtime: Arc<NodeRuntime>) {
    let Ok(Some(mut checkpoint)) = load_checkpoint() else {
        return;
    };
    let recovered = checkpoint
        .active_task_ids
        .iter()
        .filter_map(|task_id| {
            runtime
                .update_recovery
                .receipt_for_task(task_id)
                .ok()
                .flatten()
                .filter(|receipt| auto_resume_state(receipt.state))
                .map(|receipt| (task_id.clone(), receipt.active_task_id().to_string()))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let changed = recover_checkpoint_state(
        &mut checkpoint,
        &recovered,
        &crate::node_agent_release_identity::current(),
    );
    if changed {
        let _ = save_checkpoint(&checkpoint);
    }
    let update_id = checkpoint.update_id.clone();
    if checkpoint_needs_startup_reconciler(&checkpoint) {
        spawn_startup_checkpoint_reconciler(runtime, update_id);
    }
}

fn checkpoint_needs_startup_reconciler(checkpoint: &RestartCheckpoint) -> bool {
    checkpoint.state == "resume_required"
}

fn auto_resume_state(state: crate::node_agent_update_recovery::UpdateRecoveryState) -> bool {
    use crate::node_agent_update_recovery::UpdateRecoveryState;
    matches!(
        state,
        UpdateRecoveryState::Reattaching
            | UpdateRecoveryState::ResumeCreated
            | UpdateRecoveryState::Resumed
            | UpdateRecoveryState::Verified
    )
}

fn recover_checkpoint_state(
    checkpoint: &mut RestartCheckpoint,
    recovered: &std::collections::HashMap<String, String>,
    current_release_identity: &str,
) -> bool {
    if matches!(checkpoint.state.as_str(), "applying" | "restart_scheduled")
        && checkpoint
            .target_release_identity
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|target| !restart_target_matches(target, current_release_identity))
    {
        checkpoint.message = format!(
            "from_release 运行时已重新启动，仍等待目标版本 {} 安装；未将旧进程标记为目标版本在线。",
            checkpoint
                .target_release_identity
                .as_deref()
                .unwrap_or_default()
        );
        checkpoint.updated_at_ms = now_ms();
        return true;
    }
    match checkpoint.state.as_str() {
        "applying" | "restart_scheduled" => checkpoint.transition(
            "runtime_online",
            "节点已从计划更新重启恢复；没有活跃任务被静默中断。",
        ),
        "draining" if !checkpoint.active_task_ids.is_empty() => {
            let unresolved = checkpoint
                .active_task_ids
                .iter()
                .filter(|task_id| !recovered.contains_key(*task_id))
                .cloned()
                .collect::<Vec<_>>();
            if unresolved.is_empty() {
                checkpoint.active_task_ids = checkpoint
                    .active_task_ids
                    .iter()
                    .filter_map(|task_id| recovered.get(task_id).cloned())
                    .collect();
                checkpoint.transition(
                    "resumed",
                    "节点已用 update_recovery v1 回执自动接回任务；无需手动 Resume。",
                );
            } else {
                checkpoint.active_task_ids = unresolved;
                checkpoint.transition(
                    "resume_required",
                    "节点在排空完成前发生了非计划重启，且没有完整自动恢复回执；现场已保留，请使用 Resume 继续任务。",
                );
            }
        }
        _ => return false,
    }
    true
}

fn restart_target_matches(target: &str, current: &str) -> bool {
    let target = target.trim();
    let current = current.trim();
    current == target || current.starts_with(&format!("{target}+"))
}

pub(crate) fn status_payload() -> Value {
    load_checkpoint()
        .ok()
        .flatten()
        .map(|checkpoint| checkpoint.payload())
        .unwrap_or_else(|| {
            json!({
                "protocol": CHECKPOINT_PROTOCOL,
                "state": "idle",
                "active_task_ids": [],
                "resume_actions": [],
            })
        })
}

/// Return stale task ids only when they were classified by the live runtime
/// for the exact update that the standalone installer is about to apply.
pub(crate) fn confirmed_stale_registry_tasks_for_update(
    target_git_sha: &str,
) -> anyhow::Result<std::collections::HashSet<String>> {
    let Some(checkpoint) = load_checkpoint()? else {
        return Ok(std::collections::HashSet::new());
    };
    Ok(confirmed_stale_registry_tasks(&checkpoint, target_git_sha))
}

/// A durable cancel with no executor may supplement the stale registry only
/// inside the same exact-target install window. This prevents an old drain
/// decision from authorizing an unrelated later update.
pub(crate) fn stale_cancel_proof_window_for_update(target_git_sha: &str) -> anyhow::Result<bool> {
    Ok(load_checkpoint()?.is_some_and(|checkpoint| {
        checkpoint_targets_stale_cancel_recheck_window(&checkpoint, target_git_sha)
    }))
}

fn confirmed_stale_registry_tasks(
    checkpoint: &RestartCheckpoint,
    target_git_sha: &str,
) -> std::collections::HashSet<String> {
    if !checkpoint_targets_update_install_window(checkpoint, target_git_sha) {
        return std::collections::HashSet::new();
    }
    checkpoint.stale_registry_task_ids.iter().cloned().collect()
}

fn checkpoint_targets_update_install_window(
    checkpoint: &RestartCheckpoint,
    target_git_sha: &str,
) -> bool {
    if checkpoint.protocol != CHECKPOINT_PROTOCOL
        || !matches!(checkpoint.state.as_str(), "applying" | "restart_scheduled")
    {
        return false;
    }
    checkpoint_target_matches(checkpoint, target_git_sha)
}

fn checkpoint_targets_stale_cancel_recheck_window(
    checkpoint: &RestartCheckpoint,
    target_git_sha: &str,
) -> bool {
    if checkpoint.protocol != CHECKPOINT_PROTOCOL
        || !matches!(
            checkpoint.state.as_str(),
            "draining" | "applying" | "restart_scheduled"
        )
    {
        return false;
    }
    checkpoint_target_matches(checkpoint, target_git_sha)
}

fn checkpoint_target_matches(checkpoint: &RestartCheckpoint, target_git_sha: &str) -> bool {
    let Some(target) = checkpoint
        .target_release_identity
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    target == target_git_sha
        || target
            .rsplit_once('+')
            .is_some_and(|(_, git_sha)| git_sha == target_git_sha)
}

async fn apply_update(
    cloud_http_url: &str,
    download_url: Option<&str>,
    mut checkpoint: RestartCheckpoint,
) -> Result<Value, String> {
    match crate::node_agent_client_maintenance::push_update_from_server_now(
        cloud_http_url,
        download_url,
        checkpoint.target_release_identity.as_deref(),
    )
    .await
    {
        Ok(message) => {
            checkpoint.transition("restart_scheduled", &message);
            save_checkpoint(&checkpoint)?;
            Ok(json!({
                "ok": true,
                "deferred": false,
                "message": message,
                "restart_recovery": checkpoint.payload(),
            }))
        }
        Err(error) => {
            checkpoint.transition("failed", &error);
            let _ = save_checkpoint(&checkpoint);
            Err(error)
        }
    }
}

fn checkpoint_path() -> PathBuf {
    super::state_path().with_file_name("local-supervision-restart.json")
}

fn save_checkpoint(checkpoint: &RestartCheckpoint) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(checkpoint).map_err(|error| error.to_string())?;
    crate::node_agent_atomic_file::write(&checkpoint_path(), &bytes)
        .map_err(|error| error.to_string())
}

fn load_checkpoint() -> anyhow::Result<Option<RestartCheckpoint>> {
    let path = checkpoint_path();
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}

fn drain_running() -> &'static AtomicBool {
    static RUNNING: OnceLock<AtomicBool> = OnceLock::new();
    RUNNING.get_or_init(|| AtomicBool::new(false))
}

fn drain_admission_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
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

    #[test]
    fn checkpoint_exposes_one_click_resume_without_tokens() {
        let checkpoint =
            RestartCheckpoint::draining("test", vec!["local-task-1".to_string()], None, None);
        let payload = checkpoint.payload();
        assert_eq!(payload["state"], "draining");
        assert_eq!(payload["resume_actions"], json!([]));
        assert!(!payload.to_string().to_ascii_lowercase().contains("token"));
    }

    #[test]
    fn startup_recovery_distinguishes_drained_and_interrupted_updates() {
        let mut applying = RestartCheckpoint::draining("test", Vec::new(), None, None);
        applying.transition("applying", "ready");
        assert!(recover_checkpoint_state(
            &mut applying,
            &Default::default(),
            "test-release"
        ));
        assert_eq!(applying.state, "runtime_online");

        let mut scheduled = RestartCheckpoint::draining("test", Vec::new(), None, None);
        scheduled.transition("restart_scheduled", "ready");
        assert!(recover_checkpoint_state(
            &mut scheduled,
            &Default::default(),
            "test-release"
        ));
        assert_eq!(scheduled.state, "runtime_online");

        let mut interrupted =
            RestartCheckpoint::draining("test", vec!["local-task-2".to_string()], None, None);
        assert!(recover_checkpoint_state(
            &mut interrupted,
            &Default::default(),
            "test-release"
        ));
        assert_eq!(interrupted.state, "resume_required");
        assert!(interrupted.message.contains("Resume"));
        assert_eq!(
            interrupted.payload()["resume_actions"][0]["action"],
            "Resume"
        );
    }

    #[test]
    fn startup_checkpoint_does_not_override_completed_v1_auto_resume() {
        let mut checkpoint =
            RestartCheckpoint::draining("test", vec!["local-original".to_string()], None, None);
        let recovered = std::collections::HashMap::from([(
            "local-original".to_string(),
            "local-resume-generation-2".to_string(),
        )]);
        assert!(recover_checkpoint_state(
            &mut checkpoint,
            &recovered,
            "test-release"
        ));
        assert_eq!(checkpoint.state, "resumed");
        assert_eq!(checkpoint.active_task_ids, ["local-resume-generation-2"]);
        assert_eq!(checkpoint.payload()["resume_actions"], json!([]));
        assert!(checkpoint.message.contains("无需手动 Resume"));
    }

    #[test]
    fn startup_checkpoint_requires_execution_ownership_not_a_stale_running_label() {
        assert_eq!(
            startup_checkpoint_disposition(Some("running"), false),
            StartupCheckpointDisposition::Recoverable
        );
        assert_eq!(
            startup_checkpoint_disposition(Some("resume_required"), false),
            StartupCheckpointDisposition::Recoverable
        );
        assert_eq!(
            startup_checkpoint_disposition(None, false),
            StartupCheckpointDisposition::Recoverable
        );
        assert_eq!(
            startup_checkpoint_disposition(Some("running"), true),
            StartupCheckpointDisposition::Blocking
        );
        assert_eq!(
            startup_checkpoint_disposition(Some("done"), true),
            StartupCheckpointDisposition::Blocking
        );
    }

    #[test]
    fn persisted_resume_required_checkpoint_still_needs_startup_reconciliation() {
        let mut checkpoint =
            RestartCheckpoint::draining("test", vec!["old-task".to_string()], None, None);
        checkpoint.transition("resume_required", "old state");

        assert!(!recover_checkpoint_state(
            &mut checkpoint,
            &Default::default(),
            "test-release"
        ));
        assert!(checkpoint_needs_startup_reconciler(&checkpoint));
    }

    #[test]
    fn from_release_restart_waits_for_target_without_reporting_online() {
        let mut checkpoint = RestartCheckpoint::draining(
            "test",
            Vec::new(),
            None,
            Some("0.4.0+targetsha".to_string()),
        );
        checkpoint.transition("restart_scheduled", "ready");

        assert!(recover_checkpoint_state(
            &mut checkpoint,
            &Default::default(),
            "0.3.9+fromsha"
        ));

        assert_eq!(checkpoint.state, "restart_scheduled");
        assert!(checkpoint.message.contains("等待目标版本"));
        assert!(!checkpoint.message.contains("runtime_online"));
    }

    #[test]
    fn stale_registry_proof_is_bound_to_exact_target_and_applying_state() {
        let mut checkpoint = RestartCheckpoint::draining(
            "test",
            Vec::new(),
            None,
            Some("0.3.69+targetsha".to_string()),
        );
        checkpoint.stale_registry_task_ids = vec!["stale-evolution".to_string()];
        assert!(confirmed_stale_registry_tasks(&checkpoint, "targetsha").is_empty());
        assert!(!checkpoint_targets_update_install_window(
            &checkpoint,
            "targetsha"
        ));
        assert!(checkpoint_targets_stale_cancel_recheck_window(
            &checkpoint,
            "targetsha"
        ));
        assert!(!checkpoint_targets_stale_cancel_recheck_window(
            &checkpoint,
            "other-sha"
        ));

        checkpoint.transition("applying", "installer active");
        assert!(checkpoint_targets_update_install_window(
            &checkpoint,
            "targetsha"
        ));
        assert!(checkpoint_targets_stale_cancel_recheck_window(
            &checkpoint,
            "targetsha"
        ));
        assert_eq!(
            confirmed_stale_registry_tasks(&checkpoint, "targetsha"),
            std::collections::HashSet::from(["stale-evolution".to_string()])
        );

        checkpoint.transition("restart_scheduled", "ready");
        assert!(checkpoint_targets_update_install_window(
            &checkpoint,
            "targetsha"
        ));
        assert_eq!(
            confirmed_stale_registry_tasks(&checkpoint, "targetsha"),
            std::collections::HashSet::from(["stale-evolution".to_string()])
        );
        assert!(confirmed_stale_registry_tasks(&checkpoint, "other-sha").is_empty());
        assert!(!checkpoint_targets_update_install_window(
            &checkpoint,
            "other-sha"
        ));

        checkpoint.transition("resume_required", "wrong phase");
        assert!(!checkpoint_targets_stale_cancel_recheck_window(
            &checkpoint,
            "targetsha"
        ));

        checkpoint.protocol = "wrong.protocol".to_string();
        checkpoint.transition("draining", "wrong identity");
        assert!(!checkpoint_targets_stale_cancel_recheck_window(
            &checkpoint,
            "targetsha"
        ));
    }

    #[test]
    fn drain_only_allows_exact_target_proven_stale_cancel() {
        assert_eq!(
            drain_task_disposition("cancel_requested", true, true, true),
            DrainTaskDisposition::SafeStaleCancel
        );
        for status in ["running", "recovering", "reattaching", "resume_required"] {
            assert_eq!(
                drain_task_disposition(status, true, true, true),
                DrainTaskDisposition::Blocking,
                "{status} must remain fail-closed"
            );
        }
        assert_eq!(
            drain_task_disposition("cancel_requested", false, true, true),
            DrainTaskDisposition::Blocking,
            "a non-exact target must not consume stale cancellation proof"
        );
        assert_eq!(
            drain_task_disposition("cancel_requested", true, false, true),
            DrainTaskDisposition::Blocking,
            "an incomplete runtime inventory must block"
        );
        assert_eq!(
            drain_task_disposition("cancel_requested", true, true, false),
            DrainTaskDisposition::Blocking,
            "fresh handles, live sidecars, or incomplete provenance invalidate proof"
        );
    }

    #[test]
    fn repeated_broadcast_distinguishes_same_and_new_exact_targets() {
        let checkpoint = RestartCheckpoint::draining(
            "cloud_broadcast",
            vec!["local-task".to_string()],
            Some("https://example.invalid/node.exe".to_string()),
            Some("0.3.69+51841351542f7cc3".to_string()),
        );
        assert!(update_request_matches(
            &checkpoint,
            "cloud_broadcast",
            Some("https://example.invalid/node.exe"),
            Some("0.3.69+51841351542f7cc3")
        ));
        assert!(!update_request_matches(
            &checkpoint,
            "cloud_broadcast",
            Some("https://example.invalid/node.exe"),
            Some("0.3.70+aaaaaaaaaaaaaaaa")
        ));
        assert!(!update_request_matches(
            &checkpoint,
            "local_admin",
            Some("https://example.invalid/node.exe"),
            Some("0.3.69+51841351542f7cc3")
        ));
        assert!(admission_exact_target(Some("0.3.70+aaaaaaaaaaaaaaaa")));
        assert!(!admission_exact_target(Some("0.3.70")));
        assert!(!admission_exact_target(Some("latest+not-a-sha")));
    }

    #[test]
    fn injected_database_read_error_is_not_an_empty_active_set() {
        let root = std::env::temp_dir().join(format!(
            "restart-drain-db-error-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let store = crate::node_agent_local_task_store::LocalTaskStore::new(&root);
        let error = load_drain_candidates(&store)
            .expect_err("a DB open failure must fail closed instead of returning zero blockers");
        assert!(error
            .to_string()
            .contains("durable supervised task query failed"));
        let _ = std::fs::remove_dir_all(root);
    }
}
