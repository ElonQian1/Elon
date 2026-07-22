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
    let (orphan_rows_reconciled, orphan_reconcile_error) =
        match crate::node_agent_local_task_orphan_reconcile::reconcile_once(runtime.as_ref()).await
        {
            Ok(count) => (count, None),
            Err(error) => (0, Some(error.to_string())),
        };
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
        let ambiguous_recovery_receipts = recovery_receipt_count > 1
            && runtime
                .update_recovery
                .receipt_for_task(&task.task_id)
                .is_err();
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
        let live_journal_process = snapshot
            .record
            .as_ref()
            .map(crate::node_agent_local_task_orphan_reconcile::recorded_process_is_live)
            .transpose()?
            .unwrap_or(false);
        let cancel_intent_persisted = snapshot
            .record
            .as_ref()
            .and_then(|record| record.cancel_intent.as_ref())
            .is_some();
        let cancel_side_effect_committed = snapshot
            .record
            .as_ref()
            .and_then(|record| record.cancel_intent.as_ref())
            .and_then(|intent| intent.side_effect.as_ref())
            .is_some();
        let resumable_checkpoint =
            task.status == "resume_required" && task.finished_at_ms.is_some();
        let durable_cancelled = task.status == "cancel_requested" && cancel_intent_persisted;
        let persisted_inactive = resumable_checkpoint || durable_cancelled;
        let no_execution_owner =
            !fresh_runtime_handle && !live_sidecar && !replayable_sidecar && !live_journal_process;
        let no_unsafe_wait = pending_approval_ids.is_empty() && non_repeatable_action.is_none();
        let excluded = persisted_inactive
            && no_execution_owner
            && no_unsafe_wait
            && !ambiguous_recovery_receipts;
        let reason = classification_reason(
            excluded,
            persisted_inactive,
            durable_cancelled,
            cancel_side_effect_committed,
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
            live_journal_process,
            pending_approval_ids,
            non_repeatable_action,
            terminal_recovery_receipt,
            recovery_receipt_count,
            terminal_recovery_receipt_count,
            ambiguous_recovery_receipts,
            cancel_intent_persisted,
            cancel_side_effect_committed,
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
            "one or more update candidates retain live ownership or unsafe audit evidence"
                .to_string()
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
        "orphan_rows_reconciled": orphan_rows_reconciled,
        "orphan_reconcile_error": orphan_reconcile_error,
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
    persisted_inactive: bool,
    durable_cancelled: bool,
    cancel_side_effect_committed: bool,
    no_execution_owner: bool,
    no_unsafe_wait: bool,
    ambiguous_recovery_receipts: bool,
    terminal_receipt: bool,
    resume_eligible: Option<bool>,
) -> String {
    if excluded {
        return if durable_cancelled {
            if cancel_side_effect_committed {
                "durable cancel side effect is committed and no execution owner or unsafe wait remains"
            } else {
                "durable cancel intent has no remaining execution owner or unsafe wait"
            }
        } else if terminal_receipt {
            "resume_required checkpoint has compatible terminal recovery receipts and no execution owner or unsafe wait remains"
        } else {
            "resume_required checkpoint has no execution owner or unsafe wait and remains available for explicit Resume"
        }.to_string();
    }
    if !persisted_inactive {
        return if !cancel_side_effect_committed && resume_eligible.is_none() {
            "status does not prove a durable resume checkpoint or committed cancellation"
        } else {
            "task is not a durable inactive resume/cancel checkpoint"
        }
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
    "persisted inactive task remains fail-closed for an unclassified reason".to_string()
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
    use super::{classification_reason, recovery_receipt_evidence, stable_reconcile_id};
    use crate::node_agent_update_recovery::{
        UpdateGateTaskClassification, UpdateRecoveryLedger, UpdateRecoveryReceipt,
        UpdateRecoveryState,
    };

    #[test]
    fn unknown_resume_evidence_stays_fail_closed() {
        assert!(
            classification_reason(false, false, false, false, true, true, false, false, None)
                .contains("does not prove")
        );
    }

    #[test]
    fn active_execution_owner_stays_blocking() {
        assert!(classification_reason(
            false,
            true,
            false,
            false,
            false,
            true,
            false,
            true,
            Some(false)
        )
        .contains("sidecar"));
    }

    #[test]
    fn duplicate_recovery_receipts_stay_blocking_without_aborting_reconcile() {
        assert!(classification_reason(
            false,
            true,
            false,
            false,
            true,
            true,
            true,
            false,
            Some(false)
        )
        .contains("ambiguous"));

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
    fn durable_cancel_intent_is_excluded_after_execution_ownership_is_gone() {
        assert!(
            classification_reason(true, true, true, true, true, true, false, false, None)
                .contains("cancel side effect")
        );
        assert!(
            classification_reason(true, true, true, false, true, true, false, false, None)
                .contains("cancel intent")
        );
        assert!(
            classification_reason(false, false, false, false, true, true, false, false, None)
                .contains("does not prove")
        );
    }

    #[test]
    fn reconcile_id_is_idempotent_and_live_ownership_changes_the_audit() {
        let inactive = UpdateGateTaskClassification {
            task_id: "task-a".to_string(),
            status: "resume_required".to_string(),
            finished_at_ms: Some(10),
            excluded_from_install_blockers: true,
            reason: "durable inactive checkpoint".to_string(),
            ..Default::default()
        };
        let first = stable_reconcile_id("release-a", std::slice::from_ref(&inactive)).unwrap();
        let second = stable_reconcile_id("release-a", std::slice::from_ref(&inactive)).unwrap();
        assert_eq!(
            first, second,
            "consecutive reconcile passes must be idempotent"
        );

        let mut live = inactive;
        live.fresh_runtime_handle = true;
        live.excluded_from_install_blockers = false;
        live.reason = "fresh runtime handle still owns the task".to_string();
        assert_ne!(
            first,
            stable_reconcile_id("release-a", &[live]).unwrap(),
            "a real live handle must remain visible to the install audit"
        );
    }
}
