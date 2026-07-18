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
    message: String,
}

impl RestartCheckpoint {
    fn draining(source: &str, task_ids: Vec<String>, download_url: Option<String>) -> Self {
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
            "message": self.message,
            "resume_actions": resume_actions,
        })
    }
}

pub(crate) async fn schedule_update(
    runtime: Arc<NodeRuntime>,
    source: &str,
    download_url: Option<String>,
) -> Result<Value, String> {
    let active = active_supervised_task_ids(runtime.as_ref()).await;
    let cloud_http_url = runtime.cloud_http_url();
    if active.is_empty() {
        let mut checkpoint = RestartCheckpoint::draining(source, Vec::new(), download_url.clone());
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
    let mut checkpoint = RestartCheckpoint::draining(source, active, download_url);
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
            let active = active_supervised_task_ids(runtime_for_drain.as_ref()).await;
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
    if !recover_checkpoint_state(&mut checkpoint, &recovered) {
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
) -> bool {
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

async fn active_supervised_task_ids(runtime: &NodeRuntime) -> Vec<String> {
    let active = runtime.active_cli_prompts.views_without_approvals().await;
    active
        .into_iter()
        .filter_map(|task| {
            crate::node_agent_local_task_supervision::load_supervision_state(
                &runtime.task_journal,
                &task.req_id,
            )
            .ok()
            .filter(|state| state.enabled)
            .map(|_| task.req_id)
        })
        .collect()
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
            RestartCheckpoint::draining("test", vec!["local-task-1".to_string()], None);
        let payload = checkpoint.payload();
        assert_eq!(payload["state"], "draining");
        assert_eq!(payload["resume_actions"], json!([]));
        assert!(!payload.to_string().to_ascii_lowercase().contains("token"));
    }

    #[test]
    fn startup_recovery_distinguishes_drained_and_interrupted_updates() {
        let mut applying = RestartCheckpoint::draining("test", Vec::new(), None);
        applying.transition("applying", "ready");
        assert!(recover_checkpoint_state(&mut applying, &Default::default()));
        assert_eq!(applying.state, "runtime_online");

        let mut scheduled = RestartCheckpoint::draining("test", Vec::new(), None);
        scheduled.transition("restart_scheduled", "ready");
        assert!(recover_checkpoint_state(
            &mut scheduled,
            &Default::default()
        ));
        assert_eq!(scheduled.state, "runtime_online");

        let mut interrupted =
            RestartCheckpoint::draining("test", vec!["local-task-2".to_string()], None);
        assert!(recover_checkpoint_state(
            &mut interrupted,
            &Default::default()
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
            RestartCheckpoint::draining("test", vec!["local-original".to_string()], None);
        let recovered = std::collections::HashMap::from([(
            "local-original".to_string(),
            "local-resume-generation-2".to_string(),
        )]);
        assert!(recover_checkpoint_state(&mut checkpoint, &recovered));
        assert_eq!(checkpoint.state, "resumed");
        assert_eq!(checkpoint.active_task_ids, ["local-resume-generation-2"]);
        assert_eq!(checkpoint.payload()["resume_actions"], json!([]));
        assert!(checkpoint.message.contains("无需手动 Resume"));
    }
}
