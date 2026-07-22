//! Durable update checkpoints captured immediately before the Windows runtime stops.

use std::{collections::HashSet, path::Path};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::{
    git_command_error::{git_failure_message, git_spawn_context},
    node_agent_local_task_supervision::{load_supervision_contract, SUPERVISION_PROTOCOL},
    node_agent_update_recovery::{
        ReleaseIdentity, UpdateInstallGate, UpdateRecoveryReceipt, UpdateRecoveryState,
        UpdateRecoveryStore, WorkspaceGitFingerprint,
    },
};

const RECOVERY_DEADLINE_MS: u128 = 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct UpdateCheckpointDecision {
    pub(crate) active_foreground_task_ids: Vec<String>,
    pub(crate) checkpointed_task_ids: Vec<String>,
    pub(crate) live_execution_task_ids: Vec<String>,
}

impl UpdateCheckpointDecision {
    pub(crate) fn install_may_proceed(&self) -> bool {
        self.live_execution_task_ids.is_empty()
            && self
                .active_foreground_task_ids
                .iter()
                .all(|task_id| self.checkpointed_task_ids.contains(task_id))
    }
}

pub(crate) fn checkpoint_downloaded_update(
    version_file: &Path,
    remote_text: &str,
    fresh_runtime_handle_task_ids: &HashSet<String>,
) -> Result<UpdateCheckpointDecision> {
    let target: serde_json::Value =
        serde_json::from_str(remote_text).context("解析更新版本身份")?;
    let to_git_sha = target
        .get("gitSha")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("更新版本缺少 gitSha，不能建立恢复事务")?;
    let from_git_sha = read_version_git_sha(version_file).unwrap_or_default();
    let store = UpdateRecoveryStore::default();
    let mut confirmed_stale =
        crate::node_agent_restart_drain::confirmed_stale_registry_tasks_for_update(to_git_sha)?;
    let local_tasks = crate::node_agent_local_task_store::LocalTaskStore::default();
    let journal = crate::node_agent_task_journal::TaskJournal::default();
    let sidecars = crate::node_agent_cli_sidecar::CliSidecarRegistry::default();
    if crate::node_agent_restart_drain::stale_cancel_proof_window_for_update(to_git_sha)? {
        confirmed_stale.extend(proven_stale_cancelled_tasks(
            &local_tasks,
            &journal,
            &sidecars,
            Some(fresh_runtime_handle_task_ids),
        )?);
    }
    let decision = checkpoint_active_update_transactions(
        &store,
        &local_tasks,
        &journal,
        &sidecars,
        &from_git_sha,
        to_git_sha,
        &confirmed_stale,
        fresh_runtime_handle_task_ids,
    )?;
    if !decision.checkpointed_task_ids.is_empty() {
        info!(count = decision.checkpointed_task_ids.len(), %to_git_sha, "已保存节点更新任务恢复检查点");
    }
    let may_proceed = decision.install_may_proceed();
    store.update_install_gate(UpdateInstallGate {
        phase: if may_proceed {
            "checkpoint_saved".to_string()
        } else {
            "deferred_active_foreground".to_string()
        },
        target_git_sha: to_git_sha.to_string(),
        active_foreground_task_ids: decision.active_foreground_task_ids.clone(),
        safe_checkpoint_count: decision.checkpointed_task_ids.len(),
        capability: "local_update_gate_v1".to_string(),
        reason: (!may_proceed).then(|| {
            if decision.live_execution_task_ids.is_empty() {
                "active foreground task has no complete durable recovery checkpoint".to_string()
            } else {
                "active sidecar or fresh runtime handle still owns an update candidate".to_string()
            }
        }),
        updated_at_ms: crate::node_agent_cli_sidecar::now_ms(),
    })?;
    Ok(decision)
}

fn proven_stale_cancelled_tasks(
    local_tasks: &crate::node_agent_local_task_store::LocalTaskStore,
    journal: &crate::node_agent_task_journal::TaskJournal,
    sidecars: &crate::node_agent_cli_sidecar::CliSidecarRegistry,
    fresh_runtime_task_ids: Option<&HashSet<String>>,
) -> Result<HashSet<String>> {
    let Some(fresh_runtime_task_ids) = fresh_runtime_task_ids else {
        return Ok(HashSet::new());
    };
    let mut proven = HashSet::new();
    for task in local_tasks
        .list_update_install_candidates()?
        .into_iter()
        .filter(|task| matches!(task.status.as_str(), "cancel_requested" | "resume_required"))
    {
        let contract = load_supervision_contract(journal, &task.task_id)?;
        if !contract
            .as_ref()
            .is_some_and(|contract| contract.protocol == SUPERVISION_PROTOCOL)
            || fresh_runtime_task_ids.contains(&task.task_id)
        {
            continue;
        }
        let snapshot = journal.snapshot(&task.task_id, 0, 1)?;
        if snapshot
            .record
            .as_ref()
            .and_then(|record| record.cancel_intent.as_ref())
            .is_none()
        {
            continue;
        }
        if sidecars
            .session_for_task(&task.task_id)?
            .as_ref()
            .is_some_and(|session| session.recorded_process_is_live())
        {
            continue;
        }
        proven.insert(task.task_id);
    }
    Ok(proven)
}

fn checkpoint_active_update_transactions(
    store: &UpdateRecoveryStore,
    local_tasks: &crate::node_agent_local_task_store::LocalTaskStore,
    journal: &crate::node_agent_task_journal::TaskJournal,
    sidecars: &crate::node_agent_cli_sidecar::CliSidecarRegistry,
    from_git_sha: &str,
    to_git_sha: &str,
    confirmed_stale_registry_tasks: &HashSet<String>,
    fresh_runtime_handle_task_ids: &HashSet<String>,
) -> Result<UpdateCheckpointDecision> {
    let update_id = stable_update_id(to_git_sha);
    let mut decision = UpdateCheckpointDecision::default();
    for task in local_tasks.list_update_install_candidates()? {
        // A confirmed-stale registry entry is historical evidence. Re-read the
        // sidecar registry and bind the install decision to the runtime's
        // current handle inventory before allowing that evidence to suppress a
        // cancel_requested task.
        let sidecar = sidecars.session_for_task(&task.task_id)?;
        let live_sidecar = sidecar
            .as_ref()
            .is_some_and(|session| session.recorded_process_is_live());
        let live_execution = live_sidecar || fresh_runtime_handle_task_ids.contains(&task.task_id);
        if live_execution {
            decision.live_execution_task_ids.push(task.task_id.clone());
        }
        let persisted_inactive =
            matches!(task.status.as_str(), "cancel_requested" | "resume_required");
        if persisted_inactive
            && confirmed_stale_registry_tasks.contains(&task.task_id)
            && !live_execution
        {
            continue;
        }
        if persisted_inactive {
            decision
                .active_foreground_task_ids
                .push(task.task_id.clone());
            continue;
        }
        let contract = load_supervision_contract(journal, &task.task_id)?;
        if contract.as_ref().is_some_and(|contract| {
            contract.protocol == SUPERVISION_PROTOCOL
                && contract.task_role == "post_task_improvement"
        }) {
            let audit = homecli_proto::CancelRequestAudit::now(
                "node_agent",
                "self_evolution_scheduler",
                "yield_for_node_update",
            )
            .with_interruption_source(homecli_proto::InterruptionSource::UpdaterApply);
            let recorded = sidecars
                .record_cancel_command_with_audit(&task.task_id, &audit)
                .with_context(|| format!("persist updater cancel audit for {}", task.task_id))?;
            anyhow::ensure!(
                recorded,
                "updater refused to interrupt self evolution {} without a durable sidecar audit",
                task.task_id
            );
            if live_execution {
                decision
                    .active_foreground_task_ids
                    .push(task.task_id.clone());
            }
            continue;
        }
        decision
            .active_foreground_task_ids
            .push(task.task_id.clone());
        let Some(contract) = contract else {
            continue;
        };
        if contract.protocol != SUPERVISION_PROTOCOL {
            continue;
        }
        let original_task_id = if contract.task_role == "resume_original" {
            contract.parent_task_id.as_deref().unwrap_or(&task.task_id)
        } else {
            &task.task_id
        };
        let root_task_id = contract.root_task_id.as_deref().unwrap_or(original_task_id);
        let snapshot = journal.snapshot(&task.task_id, 0, 200)?;
        let workspace_path = sidecar
            .as_ref()
            .and_then(|session| session.cwd.as_deref())
            .or_else(|| {
                snapshot
                    .record
                    .as_ref()
                    .and_then(|record| record.cwd.as_deref())
            })
            .unwrap_or(&task.workspace_path)
            .to_string();
        let mut receipt =
            UpdateRecoveryReceipt::planned(&update_id, root_task_id, original_task_id);
        receipt.parent_task_id = contract.parent_task_id.clone();
        if contract.task_role == "resume_original" {
            receipt.resume_task_id = Some(task.task_id.clone());
        }
        receipt.from_release = ReleaseIdentity {
            version: crate::node_agent_release_identity::current(),
            git_sha: from_git_sha.to_string(),
        };
        receipt.to_release = ReleaseIdentity {
            version: String::new(),
            git_sha: to_git_sha.to_string(),
        };
        receipt.codex_session_id = snapshot
            .record
            .as_ref()
            .and_then(|record| record.codex_session_id.clone());
        receipt.codex_session_scope = snapshot
            .record
            .as_ref()
            .and_then(|record| record.codex_session_scope_key.clone());
        let replayable_sidecar = sidecar.as_ref().is_some_and(|session| {
            session.can_replay_output_at(crate::node_agent_cli_sidecar::now_ms())
        });
        if let Some(sidecar) = sidecar {
            receipt.sidecar_session_id = Some(sidecar.session_id);
            receipt.sidecar_output_offset = sidecar.output_offset;
            receipt.sidecar_output_sequence = sidecar.output_sequence;
        }
        receipt.journal_cursor = snapshot.last_event_seq as u64;
        receipt.workspace = fingerprint_workspace(Path::new(&workspace_path));
        preserve_platform_workspace_identity(
            &mut receipt.workspace,
            task.workspace_status.as_ref(),
        );
        receipt.recovery_policy.deadline_ms =
            Some(crate::node_agent_cli_sidecar::now_ms() + RECOVERY_DEADLINE_MS);
        receipt.safety.pending_approval_ids = snapshot.approvals.pending_approval_ids();
        receipt.safety.non_repeatable_action = incomplete_non_repeatable_action(&snapshot.events);
        receipt.safety.journal_event_count = snapshot.last_event_seq;
        receipt.safety.evidence_complete = snapshot.record.is_some()
            && receipt.workspace.has_sufficient_identity()
            && (replayable_sidecar || receipt.codex_session_id.is_some())
            && receipt.safety.pending_approval_ids.is_empty()
            && receipt.safety.non_repeatable_action.is_none();
        receipt.transition(UpdateRecoveryState::Downloaded, Some("update downloaded"))?;
        receipt.transition(
            UpdateRecoveryState::CheckpointSaved,
            Some("task checkpoint persisted"),
        )?;
        receipt.transition(
            UpdateRecoveryState::Applying,
            Some("runtime update applying"),
        )?;
        let saved = store.insert_if_absent(receipt)?;
        if saved.safety.evidence_complete {
            decision.checkpointed_task_ids.push(task.task_id);
        }
    }
    Ok(decision)
}

pub(crate) fn fingerprint_workspace(path: &Path) -> WorkspaceGitFingerprint {
    let workspace_path = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string();
    let git_head = git_output(path, &["rev-parse", "HEAD"]);
    let status = git_output(path, &["status", "--porcelain=v2", "--branch"]);
    WorkspaceGitFingerprint {
        base_workspace_path: None,
        workspace_path,
        isolated: false,
        branch: git_output(path, &["branch", "--show-current"]),
        git_head,
        git_status_sha256: status
            .as_deref()
            .map(|value| hex::encode(Sha256::digest(value.as_bytes()))),
        git_status_clean: status.as_deref().map(|value| {
            value
                .lines()
                .all(|line| line.trim().is_empty() || line.starts_with('#'))
        }),
    }
}

pub(crate) fn preserve_platform_workspace_identity(
    fingerprint: &mut WorkspaceGitFingerprint,
    status: Option<&serde_json::Value>,
) {
    let Some(status) = status
        .filter(|status| status.get("isolated").and_then(serde_json::Value::as_bool) == Some(true))
    else {
        return;
    };
    let active = status
        .get("active_workspace_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if !active
        .is_some_and(|active| same_path(Path::new(active), Path::new(&fingerprint.workspace_path)))
    {
        return;
    }
    fingerprint.base_workspace_path = status
        .get("base_workspace_path")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    fingerprint.isolated = fingerprint.base_workspace_path.is_some();
    fingerprint.branch = status
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| fingerprint.branch.clone());
}

pub(crate) fn incomplete_non_repeatable_action(
    events: &[crate::node_agent_task_journal::TaskJournalEventView],
) -> Option<String> {
    let mut pending = HashSet::new();
    for view in events {
        let event = view.event.get("event").unwrap_or(&view.event);
        let event_type = event.get("type").and_then(serde_json::Value::as_str);
        let id = event
            .get("call_id")
            .or_else(|| event.get("tool_call_id"))
            .or_else(|| event.get("id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        match event_type {
            Some("tool_call") => {
                let tool = event
                    .get("tool")
                    .or_else(|| event.get("name"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let explicit = event.get("repeatable").and_then(serde_json::Value::as_bool)
                    == Some(false)
                    || event
                        .get("non_repeatable")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true);
                if explicit
                    || ["publish", "deploy", "release", "migration", "delete"]
                        .iter()
                        .any(|needle| tool.contains(needle))
                {
                    pending.insert((id.to_string(), tool));
                }
            }
            Some("tool_result") => pending.retain(|(call_id, _)| call_id != id),
            _ => {}
        }
    }
    pending
        .into_iter()
        .next()
        .map(|(id, tool)| format!("{tool}:{id}"))
}

pub(crate) fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = match crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            warn!(
                cwd = %cwd.display(),
                command = %git_spawn_context(args),
                %error,
                "节点更新检查点无法启动 Git"
            );
            return None;
        }
    };
    if !output.status.success() {
        warn!(message = %git_failure_message(cwd, args, &output), "节点更新检查点 Git 失败");
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string(),
    )
}

fn stable_update_id(to_git_sha: &str) -> String {
    format!("node-update-{}", safe_fragment(to_git_sha, 48))
}

pub(crate) fn stable_resume_task_id(update_id: &str, original_task_id: &str) -> String {
    let digest = Sha256::digest(format!("{update_id}\0{original_task_id}").as_bytes());
    format!("local-recovery-{}", &hex::encode(digest)[..24])
}

pub(crate) fn file_sha256(path: &Path) -> Option<String> {
    Some(hex::encode(Sha256::digest(std::fs::read(path).ok()?)))
}

pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

fn safe_fragment(value: &str, limit: usize) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .take(limit)
        .collect()
}

fn read_version_git_sha(path: &Path) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(&std::fs::read(path).ok()?)
        .ok()?
        .get("gitSha")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
#[path = "node_agent_update_checkpoint_tests.rs"]
mod tests;
