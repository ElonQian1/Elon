use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::NodeRuntime;

const CHECKPOINT_PROTOCOL: &str = "elon.local_supervision_restart.v1";
const DRAIN_POLL_SECS: u64 = 3;
const ACTIVE_HANDLE_STALE_AFTER_MS: u128 = 2 * 60 * 1_000;

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
    let classification = classify_supervised_tasks(runtime.as_ref())
        .await
        .map_err(|error| format!("读取监督任务排空状态失败，已拒绝更新：{error:#}"))?;
    let active = classification.blocking;
    let cloud_http_url = runtime.cloud_http_url();
    if active.is_empty() {
        let mut checkpoint = RestartCheckpoint::draining(
            source,
            Vec::new(),
            download_url.clone(),
            target_release_identity,
        );
        checkpoint.recoverable_task_ids = classification.recoverable;
        checkpoint.stale_registry_task_ids = classification.stale;
        checkpoint.transition("applying", "没有活跃监督任务，正在安全应用更新。");
        save_checkpoint(&checkpoint)?;
        return apply_update(&cloud_http_url, download_url.as_deref(), checkpoint).await;
    }

    if drain_running()
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        let existing = load_checkpoint()
            .ok()
            .flatten()
            .map(|value| value.payload());
        return Ok(json!({
            "ok": true,
            "deferred": true,
            "restart_recovery": existing,
        }));
    }
    let mut checkpoint =
        RestartCheckpoint::draining(source, active, download_url, target_release_identity);
    checkpoint.recoverable_task_ids = classification.recoverable;
    checkpoint.stale_registry_task_ids = classification.stale;
    if let Err(error) = save_checkpoint(&checkpoint) {
        drain_running().store(false, Ordering::Release);
        return Err(error);
    }

    let response_checkpoint = checkpoint.clone();
    let runtime_for_drain = runtime.clone();
    let cloud_http_for_drain = cloud_http_url.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(DRAIN_POLL_SECS)).await;
            let classification = match classify_supervised_tasks(runtime_for_drain.as_ref()).await {
                Ok(classification) => classification,
                Err(error) => {
                    checkpoint.message =
                        format!("读取监督任务排空状态失败，更新继续保持延期：{error:#}");
                    checkpoint.updated_at_ms = now_ms();
                    let _ = save_checkpoint(&checkpoint);
                    continue;
                }
            };
            let active = classification.blocking;
            checkpoint.recoverable_task_ids = classification.recoverable;
            checkpoint.stale_registry_task_ids = classification.stale;
            if !active.is_empty() {
                checkpoint.active_task_ids = active;
                checkpoint.updated_at_ms = now_ms();
                let _ = save_checkpoint(&checkpoint);
                continue;
            }
            checkpoint.active_task_ids.clear();
            checkpoint.transition("applying", "监督任务已安全结束，正在应用延期更新。");
            let _ = save_checkpoint(&checkpoint);
            if let Err(error) = apply_update(
                &cloud_http_for_drain,
                checkpoint.download_url.as_deref(),
                checkpoint.clone(),
            )
            .await
            {
                checkpoint.transition("failed", error);
                let _ = save_checkpoint(&checkpoint);
            }
            drain_running().store(false, Ordering::Release);
            break;
        }
    });

    Ok(json!({
        "ok": true,
        "deferred": true,
        "restart_recovery": response_checkpoint.payload(),
    }))
}

pub(crate) fn recover_checkpoint_after_startup(
    update_recovery: &crate::node_agent_update_recovery::UpdateRecoveryStore,
) {
    let Ok(Some(mut checkpoint)) = load_checkpoint() else {
        return;
    };
    let recovered = checkpoint
        .active_task_ids
        .iter()
        .filter_map(|task_id| {
            update_recovery
                .receipt_for_task(task_id)
                .ok()
                .flatten()
                .filter(|receipt| auto_resume_state(receipt.state))
                .map(|receipt| (task_id.clone(), receipt.active_task_id().to_string()))
        })
        .collect::<std::collections::HashMap<_, _>>();
    if !recover_checkpoint_state(
        &mut checkpoint,
        &recovered,
        &crate::node_agent_release_identity::current(),
    ) {
        return;
    }
    let _ = save_checkpoint(&checkpoint);
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

#[derive(Default)]
struct DrainClassification {
    blocking: Vec<String>,
    recoverable: Vec<String>,
    stale: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum DrainTaskDisposition {
    Blocking,
    Recoverable,
    Stale,
}

fn drain_task_disposition(
    durable_running: bool,
    cancel_requested: bool,
    handle_fresh: bool,
    replayable_sidecar: bool,
    safe_receipt: bool,
) -> DrainTaskDisposition {
    if !durable_running {
        DrainTaskDisposition::Stale
    } else if replayable_sidecar || safe_receipt {
        DrainTaskDisposition::Recoverable
    } else if cancel_requested {
        // Cancellation is its own durable state machine. Never rewrite it to
        // Resume merely because the executor heartbeat became stale.
        DrainTaskDisposition::Blocking
    } else if handle_fresh {
        DrainTaskDisposition::Blocking
    } else {
        DrainTaskDisposition::Stale
    }
}

async fn classify_supervised_tasks(runtime: &NodeRuntime) -> anyhow::Result<DrainClassification> {
    let active = runtime.active_cli_prompts.views_without_approvals().await;
    let active = active
        .into_iter()
        .map(|task| (task.req_id.clone(), task))
        .collect::<std::collections::HashMap<_, _>>();
    let now = now_ms();
    let mut result = DrainClassification::default();
    let tasks = load_drain_candidates(&runtime.local_tasks)?;
    let mut seen = std::collections::HashSet::new();
    for task in tasks {
        seen.insert(task.task_id.clone());
        let supervised = crate::node_agent_local_task_supervision::load_supervision_state(
            &runtime.task_journal,
            &task.task_id,
        )?
        .enabled;
        if !supervised {
            continue;
        }
        let durable_running = matches!(
            task.status.as_str(),
            "running" | "recovering" | "reattaching" | "cancel_requested"
        );
        let handle_fresh = active.get(&task.task_id).is_some_and(|handle| {
            handle.control_handle_live
                && now.saturating_sub(handle.last_heartbeat_ms) <= ACTIVE_HANDLE_STALE_AFTER_MS
        });
        let sidecar = runtime.cli_sidecars.session_for_task(&task.task_id)?;
        let replayable_sidecar = sidecar
            .as_ref()
            .is_some_and(|sidecar| sidecar.can_replay_after_restart_at(now));
        let safe_receipt = runtime
            .update_recovery
            .receipt_for_task(&task.task_id)?
            .is_some_and(|receipt| {
                receipt.safety.evidence_complete
                    && matches!(
                        receipt.state,
                        crate::node_agent_update_recovery::UpdateRecoveryState::Reattaching
                            | crate::node_agent_update_recovery::UpdateRecoveryState::ResumeCreated
                            | crate::node_agent_update_recovery::UpdateRecoveryState::Resumed
                    )
            });
        match drain_task_disposition(
            durable_running,
            task.status == "cancel_requested",
            handle_fresh,
            replayable_sidecar,
            safe_receipt,
        ) {
            DrainTaskDisposition::Stale => {
                let context = json!({
                    "state": "resume_required",
                    "reason": "stale_runtime_and_sidecar",
                    "sidecar_session_id": sidecar.as_ref().map(|value| value.session_id.as_str()),
                    "sidecar_pid": sidecar.as_ref().and_then(|value| value.sidecar_pid),
                    "child_pid": sidecar.as_ref().and_then(|value| value.child_pid),
                    "journal_preserved": true,
                    "workspace_preserved": true,
                    "root_lease_preserved": true,
                });
                let reason = "监督任务没有活动运行句柄，且记录的 sidecar/CLI 进程均不存活；现场已保留并转入 Resume";
                let transitioned = runtime.local_tasks.mark_stale_sidecar_resume_required(
                    &task.task_id,
                    reason,
                    &context,
                )?;
                if transitioned {
                    runtime
                        .cli_sidecars
                        .mark_task_resume_required(&task.task_id)?;
                    runtime.active_cli_prompts.remove(&task.task_id).await;
                    crate::node_agent_local_task_supervision::record_supervision_event(
                        &runtime.task_journal,
                        &task.task_id,
                        "supervision_stale_runtime_resume_required",
                        context,
                    )?;
                    result.stale.push(task.task_id.clone());
                } else {
                    // A concurrent durable transition is not proof that the
                    // task is safe to drain. Re-read on the next pass.
                    result.blocking.push(task.task_id.clone());
                }
            }
            DrainTaskDisposition::Recoverable => result.recoverable.push(task.task_id),
            DrainTaskDisposition::Blocking => result.blocking.push(task.task_id),
        }
    }
    // A missing durable row must never turn an otherwise active supervised
    // handle into an empty blocking set. Journal read failures also propagate.
    for task in active.values().filter(|task| !seen.contains(&task.req_id)) {
        if crate::node_agent_local_task_supervision::load_supervision_state(
            &runtime.task_journal,
            &task.req_id,
        )?
        .enabled
        {
            result.blocking.push(task.req_id.clone());
        }
    }
    result.blocking.sort();
    result.recoverable.sort();
    result.stale.sort();
    Ok(result)
}

fn load_drain_candidates(
    store: &crate::node_agent_local_task_store::LocalTaskStore,
) -> anyhow::Result<Vec<crate::node_agent_local_task_store::LocalTaskRecord>> {
    store
        .list_update_candidates()
        .map_err(|error| anyhow::anyhow!("durable supervised task query failed: {error:#}"))
}

async fn apply_update(
    cloud_http_url: &str,
    download_url: Option<&str>,
    mut checkpoint: RestartCheckpoint,
) -> Result<Value, String> {
    match crate::node_agent_client_maintenance::push_update_from_server_now(
        cloud_http_url,
        download_url,
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
    fn drain_only_waits_for_genuine_uncheckpointed_execution() {
        assert_eq!(
            drain_task_disposition(true, false, true, false, false),
            DrainTaskDisposition::Blocking
        );
        assert_eq!(
            drain_task_disposition(true, false, true, true, false),
            DrainTaskDisposition::Recoverable
        );
        assert_eq!(
            drain_task_disposition(true, false, true, false, true),
            DrainTaskDisposition::Recoverable
        );
        assert_eq!(
            drain_task_disposition(true, false, false, false, false),
            DrainTaskDisposition::Stale
        );
        assert_eq!(
            drain_task_disposition(false, false, true, false, false),
            DrainTaskDisposition::Stale
        );
        assert_eq!(
            drain_task_disposition(true, true, false, false, false),
            DrainTaskDisposition::Blocking,
            "cancel_requested keeps cancellation semantics while unsettled"
        );
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
