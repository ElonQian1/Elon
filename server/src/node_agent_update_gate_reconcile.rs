//! Audited, fail-closed repair of update blockers left by terminal local tasks.

use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    node_agent_local_task_supervision::load_supervision_contract,
    node_agent_update_checkpoint::incomplete_non_repeatable_action,
    node_agent_update_recovery::{
        UpdateGateTaskClassification, UpdateInstallGate, UpdateRecoveryLedger, UpdateRecoveryState,
    },
    NodeRuntime,
};

pub(crate) async fn reconcile(runtime: Arc<NodeRuntime>) -> Result<Value> {
    let credentials = runtime.creds().await.context("节点当前没有已绑定身份")?;
    let fresh_handles = runtime
        .active_cli_prompts
        .views_without_approvals()
        .await
        .into_iter()
        .filter(|handle| handle.control_handle_live)
        .map(|handle| handle.req_id)
        .collect::<HashSet<_>>();
    let recovery_ledger = runtime.update_recovery.load()?;
    let mut classifications = Vec::new();
    for task in runtime.local_tasks.list_update_install_candidates()? {
        if task.owner_user_id != credentials.owner_user_id
            || task.agent_id != credentials.agent_id
            || task.install_id != runtime.install_id
        {
            classifications.push(blocked_identity(&task));
            continue;
        }
        let snapshot = runtime.task_journal.snapshot(&task.task_id, 0, 10_000)?;
        let sidecar = runtime.cli_sidecars.session_for_task(&task.task_id)?;
        let now = crate::node_agent_cli_sidecar::now_ms();
        let live_sidecar = sidecar
            .as_ref()
            .is_some_and(|session| session.recorded_process_is_live());
        let replayable_sidecar = sidecar
            .as_ref()
            .is_some_and(|session| session.can_replay_output_at(now));
        let pending_approval_ids = snapshot.approvals.pending_approval_ids();
        let non_repeatable_action = if snapshot.has_more {
            Some("journal_exceeds_audit_limit".to_string())
        } else {
            incomplete_non_repeatable_action(&snapshot.events)
        };
        let (recovery_receipt_count, terminal_recovery_receipt_count, terminal_recovery_receipt) =
            recovery_receipt_evidence(&recovery_ledger, &task.task_id);
        let ambiguous_recovery_receipts =
            recovery_receipt_count > 1 && terminal_recovery_receipt_count != recovery_receipt_count;
        let contract = load_supervision_contract(&runtime.task_journal, &task.task_id)?;
        let resume = crate::node_agent_local_task_resume_routes::inspect_resume_workspace_status(
            &runtime,
            &task,
            snapshot.record.as_ref(),
            contract.as_ref(),
        )
        .await;
        let resume_eligible = resume.get("eligible").and_then(Value::as_bool);
        let resume_ineligibility_proof = (resume_eligible == Some(false))
            .then(|| {
                task.workspace_status
                    .as_ref()
                    .and_then(|status| status.get("resume_blocked_reason"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|reason| !reason.is_empty())
                    .map(str::to_string)
            })
            .flatten();
        let fresh_runtime_handle = fresh_handles.contains(&task.task_id);
        let base_terminal = task.status == "resume_required" && task.finished_at_ms.is_some();
        let no_execution_owner = !fresh_runtime_handle && !live_sidecar && !replayable_sidecar;
        let no_unsafe_wait = pending_approval_ids.is_empty() && non_repeatable_action.is_none();
        let terminal_proof = terminal_exclusion_proven(
            terminal_recovery_receipt,
            resume_ineligibility_proof.as_deref(),
        );
        let excluded = base_terminal
            && no_execution_owner
            && no_unsafe_wait
            && !ambiguous_recovery_receipts
            && terminal_proof;
        let reason = classification_reason(
            excluded,
            base_terminal,
            no_execution_owner,
            no_unsafe_wait,
            ambiguous_recovery_receipts,
            terminal_recovery_receipt,
            resume_eligible,
        );
        classifications.push(UpdateGateTaskClassification {
            task_id: task.task_id,
            status: task.status,
            finished_at_ms: task.finished_at_ms,
            fresh_runtime_handle,
            live_sidecar,
            replayable_sidecar,
            pending_approval_ids,
            non_repeatable_action,
            terminal_recovery_receipt,
            recovery_receipt_count,
            terminal_recovery_receipt_count,
            ambiguous_recovery_receipts,
            resume_eligible,
            resume_ineligibility_proof,
            excluded_from_install_blockers: excluded,
            reason,
        });
    }
    classifications.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    let blockers = classifications
        .iter()
        .filter(|classification| !classification.excluded_from_install_blockers)
        .map(|classification| classification.task_id.clone())
        .collect::<Vec<_>>();
    let excluded_count = classifications.len().saturating_sub(blockers.len());
    let current = runtime.update_recovery.load()?.install_gate;
    let reconcile_id = stable_reconcile_id(&current.target_git_sha, &classifications)?;
    let now = crate::node_agent_cli_sidecar::now_ms();
    let gate = UpdateInstallGate {
        phase: if blockers.is_empty() {
            "checkpoint_saved"
        } else {
            "deferred_active_foreground"
        }
        .to_string(),
        target_git_sha: current.target_git_sha,
        active_foreground_task_ids: blockers.clone(),
        safe_checkpoint_count: current.safe_checkpoint_count,
        capability: "local_update_gate_reconcile_v1".to_string(),
        reason: (!blockers.is_empty()).then(|| {
            "one or more update candidates still lack terminal exclusion proof".to_string()
        }),
        excluded_terminal_history_count: excluded_count,
        reconcile_id: Some(reconcile_id.clone()),
        reconciled_at_ms: Some(now),
        classifications: classifications.clone(),
        updated_at_ms: now,
    };
    runtime.update_recovery.update_install_gate(gate.clone())?;
    Ok(json!({
        "ok": true,
        "schema": "elon.node_update_gate_reconcile.v1",
        "idempotency": "stable_reconcile_id",
        "reconcile_id": reconcile_id,
        "install_may_proceed": blockers.is_empty(),
        "excluded_terminal_history_count": excluded_count,
        "active_foreground_task_ids": blockers,
        "install_gate": gate,
    }))
}

fn blocked_identity(
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
) -> UpdateGateTaskClassification {
    UpdateGateTaskClassification {
        task_id: task.task_id.clone(),
        status: task.status.clone(),
        finished_at_ms: task.finished_at_ms,
        reason: "task owner/agent/install identity does not match this runtime".to_string(),
        ..UpdateGateTaskClassification::default()
    }
}

fn classification_reason(
    excluded: bool,
    base_terminal: bool,
    no_execution_owner: bool,
    no_unsafe_wait: bool,
    ambiguous_recovery_receipts: bool,
    terminal_receipt: bool,
    resume_eligible: Option<bool>,
) -> String {
    if excluded {
        return if terminal_receipt {
            "terminal resume_required task with only failed/verified recovery receipts and no remaining execution ownership"
        } else {
            "terminal resume_required task is explicitly ineligible for resume and has no remaining execution ownership"
        }.to_string();
    }
    if !base_terminal {
        return "status or finished_at does not prove a terminal resume_required history row"
            .to_string();
    }
    if !no_execution_owner {
        return "fresh runtime handle or live/replayable sidecar still owns the task".to_string();
    }
    if !no_unsafe_wait {
        return "pending approval, incomplete audit, or non-repeatable action remains".to_string();
    }
    if ambiguous_recovery_receipts {
        return "multiple update recovery receipts target the task; terminal proof is ambiguous"
            .to_string();
    }
    if resume_eligible == Some(true) {
        return "task remains eligible for resume".to_string();
    }
    "terminal recovery eligibility is unknown and no failed/verified receipt exists".to_string()
}

fn terminal_exclusion_proven(
    terminal_recovery_receipt: bool,
    resume_ineligibility_proof: Option<&str>,
) -> bool {
    terminal_recovery_receipt
        || resume_ineligibility_proof.is_some_and(|reason| !reason.trim().is_empty())
}

fn recovery_receipt_evidence(ledger: &UpdateRecoveryLedger, task_id: &str) -> (usize, usize, bool) {
    let matches = ledger
        .receipts
        .iter()
        .filter(|receipt| {
            receipt.original_task_id == task_id
                || receipt.resume_task_id.as_deref() == Some(task_id)
        })
        .collect::<Vec<_>>();
    let count = matches.len();
    let terminal_count = matches
        .iter()
        .filter(|receipt| {
            matches!(
                receipt.state,
                UpdateRecoveryState::Failed | UpdateRecoveryState::Verified
            )
        })
        .count();
    (count, terminal_count, count > 0 && terminal_count == count)
}

fn stable_reconcile_id(
    target: &str,
    classifications: &[UpdateGateTaskClassification],
) -> Result<String> {
    let bytes = serde_json::to_vec(&(target, classifications))?;
    Ok(format!(
        "reconcile-{}",
        &hex::encode(Sha256::digest(bytes))[..24]
    ))
}

#[cfg(test)]
mod tests {
    use super::{classification_reason, recovery_receipt_evidence, terminal_exclusion_proven};
    use crate::node_agent_update_recovery::{
        UpdateRecoveryLedger, UpdateRecoveryReceipt, UpdateRecoveryState,
    };

    #[test]
    fn unknown_resume_evidence_stays_fail_closed() {
        assert!(
            classification_reason(false, true, true, true, false, false, None).contains("unknown")
        );
    }

    #[test]
    fn active_execution_owner_stays_blocking() {
        assert!(
            classification_reason(false, true, false, true, false, true, Some(false))
                .contains("sidecar")
        );
    }

    #[test]
    fn duplicate_recovery_receipts_stay_blocking_without_aborting_reconcile() {
        assert!(
            classification_reason(false, true, true, true, true, false, Some(false))
                .contains("ambiguous")
        );

        let mut ledger = UpdateRecoveryLedger::default();
        let mut first = UpdateRecoveryReceipt::planned("update-a", "root-a", "task-a");
        first.state = UpdateRecoveryState::Failed;
        let mut second = UpdateRecoveryReceipt::planned("update-b", "root-b", "task-a");
        second.state = UpdateRecoveryState::Verified;
        ledger.receipts.extend([first, second]);
        assert_eq!(recovery_receipt_evidence(&ledger, "task-a"), (2, 2, true));

        ledger.receipts[1].state = UpdateRecoveryState::Paused;
        assert_eq!(recovery_receipt_evidence(&ledger, "task-a"), (2, 1, false));
    }

    #[test]
    fn generic_ineligible_without_durable_reason_is_not_terminal_proof() {
        assert!(!terminal_exclusion_proven(false, None));
        assert!(terminal_exclusion_proven(
            false,
            Some("terminal workspace snapshot was rejected")
        ));
        assert!(terminal_exclusion_proven(true, None));
    }
}
