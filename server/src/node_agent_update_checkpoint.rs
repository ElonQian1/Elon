//! Durable update checkpoints captured immediately before the Windows runtime stops.

use std::{collections::HashSet, path::Path, process::Command};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tracing::info;

use crate::{
    node_agent_local_task_supervision::{load_supervision_contract, SUPERVISION_PROTOCOL},
    node_agent_update_recovery::{
        ReleaseIdentity, UpdateRecoveryReceipt, UpdateRecoveryState, UpdateRecoveryStore,
        WorkspaceGitFingerprint,
    },
};

const RECOVERY_DEADLINE_MS: u128 = 24 * 60 * 60 * 1_000;

pub(crate) fn checkpoint_downloaded_update(version_file: &Path, remote_text: &str) -> Result<()> {
    let target: serde_json::Value =
        serde_json::from_str(remote_text).context("解析更新版本身份")?;
    let to_git_sha = target
        .get("gitSha")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("更新版本缺少 gitSha，不能建立恢复事务")?;
    let from_git_sha = read_version_git_sha(version_file).unwrap_or_default();
    let count = checkpoint_active_update_transactions(
        &UpdateRecoveryStore::default(),
        &crate::node_agent_local_task_store::LocalTaskStore::default(),
        &crate::node_agent_task_journal::TaskJournal::default(),
        &crate::node_agent_cli_sidecar::CliSidecarRegistry::default(),
        &from_git_sha,
        to_git_sha,
    )?;
    if count > 0 {
        info!(count, %to_git_sha, "已保存节点更新任务恢复检查点");
    }
    Ok(())
}

fn checkpoint_active_update_transactions(
    store: &UpdateRecoveryStore,
    local_tasks: &crate::node_agent_local_task_store::LocalTaskStore,
    journal: &crate::node_agent_task_journal::TaskJournal,
    sidecars: &crate::node_agent_cli_sidecar::CliSidecarRegistry,
    from_git_sha: &str,
    to_git_sha: &str,
) -> Result<usize> {
    let update_id = stable_update_id(to_git_sha);
    let mut saved = 0;
    for task in local_tasks.list_update_candidates()? {
        let Some(contract) = load_supervision_contract(journal, &task.task_id)? else {
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
        let sidecar = sidecars.session_for_task(&task.task_id)?;
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
        receipt.sidecar_session_id = sidecar.map(|session| session.session_id);
        receipt.journal_cursor = snapshot.last_event_seq as u64;
        receipt.workspace = fingerprint_workspace(Path::new(&workspace_path));
        receipt.recovery_policy.deadline_ms =
            Some(crate::node_agent_cli_sidecar::now_ms() + RECOVERY_DEADLINE_MS);
        receipt.safety.evidence_complete =
            snapshot.record.is_some() && receipt.workspace.has_sufficient_identity();
        receipt.safety.pending_approval_ids = snapshot.approvals.pending_approval_ids();
        receipt.safety.non_repeatable_action = incomplete_non_repeatable_action(&snapshot.events);
        receipt.safety.journal_event_count = snapshot.last_event_seq;
        receipt.transition(UpdateRecoveryState::Downloaded, Some("update downloaded"))?;
        receipt.transition(
            UpdateRecoveryState::CheckpointSaved,
            Some("task checkpoint persisted"),
        )?;
        receipt.transition(
            UpdateRecoveryState::Applying,
            Some("runtime update applying"),
        )?;
        store.insert_if_absent(receipt)?;
        saved += 1;
    }
    Ok(saved)
}

pub(crate) fn fingerprint_workspace(path: &Path) -> WorkspaceGitFingerprint {
    let workspace_path = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string();
    let git_head = git_output(path, &["rev-parse", "HEAD"]);
    let status = git_output(path, &["status", "--porcelain=v2", "--branch"]);
    WorkspaceGitFingerprint {
        workspace_path,
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
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string()
    })
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
