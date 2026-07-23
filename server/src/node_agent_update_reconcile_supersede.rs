//! Safe convergence of historical recovery receipts after a later node update.

use anyhow::Result;

use crate::node_agent_update_recovery::{
    ReleaseIdentity, UpdateRecoveryEvent, UpdateRecoveryReceipt, UpdateRecoveryState,
    UpdateRecoveryStore,
};

use super::{now_ms, release_identity_matches};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SupersedingReleaseEvidence {
    pub(super) update_id: String,
    pub(super) release: ReleaseIdentity,
    pub(super) source: &'static str,
}

pub(super) fn superseding_release_evidence(
    store: &UpdateRecoveryStore,
    receipt: &UpdateRecoveryReceipt,
    current_release: &str,
) -> Result<Option<SupersedingReleaseEvidence>> {
    let Some(current) = exact_release_identity(current_release) else {
        return Ok(None);
    };
    let ledger = store.load()?;
    if let Some(successor) = ledger
        .receipts
        .iter()
        .filter(|candidate| {
            candidate.update_id != receipt.update_id
                && !candidate.is_superseded()
                && candidate.allows_local_reconcile()
                && candidate.root_task_id == receipt.root_task_id
                && candidate.original_task_id == receipt.original_task_id
                && candidate.active_task_id() == receipt.active_task_id()
                && candidate.created_at_ms >= receipt.created_at_ms
                && chained_release_identity(&receipt.to_release, &candidate.from_release)
                && release_identity_matches(&candidate.to_release, current_release)
                && matches!(
                    candidate.state,
                    UpdateRecoveryState::Applying
                        | UpdateRecoveryState::RuntimeOnline
                        | UpdateRecoveryState::Reattaching
                        | UpdateRecoveryState::ResumeCreated
                        | UpdateRecoveryState::Resumed
                        | UpdateRecoveryState::Verified
                )
        })
        .max_by_key(|candidate| (candidate.created_at_ms, candidate.updated_at_ms))
    {
        return Ok(Some(SupersedingReleaseEvidence {
            update_id: successor.update_id.clone(),
            release: current,
            source: "successor_update_receipt",
        }));
    }

    if let Some(observation) = ledger
        .receipts
        .iter()
        .filter(|candidate| same_sidecar_release_observation(receipt, candidate, current_release))
        .max_by_key(|candidate| (candidate.created_at_ms, candidate.updated_at_ms))
    {
        return Ok(Some(SupersedingReleaseEvidence {
            update_id: observation.update_id.clone(),
            release: current,
            source: "same_sidecar_current_release_receipt",
        }));
    }

    let gate = &ledger.install_gate;
    let task_is_checkpointed = gate
        .active_foreground_task_ids
        .iter()
        .any(|task_id| task_id == receipt.active_task_id() || task_id == &receipt.original_task_id);
    if gate.target_git_sha.trim() == current.git_sha
        && matches!(gate.phase.as_str(), "checkpoint_saved" | "runtime_online")
        && gate.safe_checkpoint_count > 0
        && task_is_checkpointed
    {
        return Ok(Some(SupersedingReleaseEvidence {
            update_id: format!("node-update-{}", current.git_sha),
            release: current,
            source: "task_bound_install_gate",
        }));
    }
    Ok(None)
}

fn same_sidecar_release_observation(
    receipt: &UpdateRecoveryReceipt,
    candidate: &UpdateRecoveryReceipt,
    current_release: &str,
) -> bool {
    let sidecar_session_id = receipt.sidecar_session_id.as_deref().unwrap_or("").trim();
    !sidecar_session_id.is_empty()
        && candidate.update_id != receipt.update_id
        && candidate.update_id.starts_with("legacy-sidecar-")
        && !candidate.is_superseded()
        && candidate.allows_local_reconcile()
        && candidate.root_task_id == receipt.root_task_id
        && candidate.parent_task_id == receipt.parent_task_id
        && candidate.original_task_id == receipt.original_task_id
        && candidate.active_task_id() == receipt.active_task_id()
        && candidate.sidecar_session_id.as_deref() == Some(sidecar_session_id)
        && candidate.created_at_ms > receipt.created_at_ms
        && candidate.transport.kind == receipt.transport.kind
        && candidate.transport.protocol == receipt.transport.protocol
        && candidate.transport.auth_mode == receipt.transport.auth_mode
        && candidate.safety.journal_event_count >= receipt.safety.journal_event_count
        && !candidate.to_release.git_sha.trim().is_empty()
        && candidate.from_release.git_sha == candidate.to_release.git_sha
        && candidate.to_release.git_sha != receipt.to_release.git_sha
        && release_identity_matches(&candidate.to_release, current_release)
        && matches!(
            candidate.state,
            UpdateRecoveryState::Applying
                | UpdateRecoveryState::RuntimeOnline
                | UpdateRecoveryState::Reattaching
                | UpdateRecoveryState::ResumeCreated
                | UpdateRecoveryState::Resumed
                | UpdateRecoveryState::Verified
        )
}

fn exact_release_identity(current: &str) -> Option<ReleaseIdentity> {
    let (version, git_sha) = current.trim().rsplit_once('+')?;
    let version = version.trim();
    let git_sha = git_sha.trim();
    if version.is_empty()
        || git_sha.len() < 7
        || !git_sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    Some(ReleaseIdentity {
        version: version.to_string(),
        git_sha: git_sha.to_string(),
    })
}

fn chained_release_identity(
    previous_target: &ReleaseIdentity,
    successor_from: &ReleaseIdentity,
) -> bool {
    let previous_sha = previous_target.git_sha.trim();
    let successor_sha = successor_from.git_sha.trim();
    !previous_sha.is_empty()
        && previous_sha == successor_sha
        && (previous_target.version.trim().is_empty()
            || successor_from.version.trim().is_empty()
            || previous_target.version.trim() == successor_from.version.trim()
            || successor_from
                .version
                .trim()
                .starts_with(&format!("{}+", previous_target.version.trim())))
}

pub(super) fn record_superseded_recovery(
    store: &UpdateRecoveryStore,
    receipt: &UpdateRecoveryReceipt,
    evidence: &SupersedingReleaseEvidence,
) -> Result<bool> {
    store.update(&receipt.update_id, &receipt.original_task_id, |current| {
        if current.is_superseded() {
            anyhow::ensure!(
                current.superseded_by_update_id.as_deref() == Some(&evidence.update_id)
                    && current.superseded_by_release.as_ref() == Some(&evidence.release)
                    && current.supersede_evidence.as_deref() == Some(evidence.source),
                "update recovery receipt has conflicting supersede evidence"
            );
            return Ok(false);
        }
        anyhow::ensure!(
            receipt_may_be_superseded(current),
            "only a resumed sidecar receipt or the exact historical release-mismatch failure may be superseded"
        );
        let previous_state = current.state;
        current.superseded_by_update_id = Some(evidence.update_id.clone());
        current.superseded_by_release = Some(evidence.release.clone());
        current.supersede_evidence = Some(evidence.source.to_string());
        current.superseded_at_ms = Some(now_ms());
        let reason = format!(
            "superseded by {} at {}+{} via {}; no recovery action replayed",
            evidence.update_id,
            evidence.release.version,
            evidence.release.git_sha,
            evidence.source
        );
        if previous_state == UpdateRecoveryState::Resumed {
            current.transition(UpdateRecoveryState::Verified, Some(&reason))?;
        } else {
            let sequence = current
                .events
                .last()
                .map(|event| event.sequence + 1)
                .unwrap_or(1);
            let at_ms = now_ms();
            current.updated_at_ms = at_ms;
            current.state_reason = Some(reason.clone());
            current.events.push(UpdateRecoveryEvent {
                event_id: format!("{}:{sequence}:superseded", current.update_id),
                sequence,
                state: previous_state,
                at_ms,
                reason: Some(reason),
            });
        }
        Ok(true)
    })
}

pub(super) fn reconcile_superseded_history(
    store: &UpdateRecoveryStore,
    current_release: &str,
) -> Result<usize> {
    let receipts = store.load()?.receipts;
    let mut changed = 0;
    for receipt in receipts {
        if receipt.is_superseded() || !receipt_may_be_superseded(&receipt) {
            continue;
        }
        if let Some(evidence) = superseding_release_evidence(store, &receipt, current_release)? {
            if record_superseded_recovery(store, &receipt, &evidence)? {
                changed += 1;
            }
        }
    }
    Ok(changed)
}

fn receipt_may_be_superseded(receipt: &UpdateRecoveryReceipt) -> bool {
    if receipt.state == UpdateRecoveryState::Resumed {
        return matches!(
            receipt.resume_strategy.as_deref(),
            Some("sidecar_reattach" | "sidecar_terminal_replay")
        );
    }
    receipt.state == UpdateRecoveryState::Failed
        && receipt.resume_strategy.as_deref() == Some("sidecar_reattach")
        && receipt.safety.evidence_complete
        && receipt.safety.pending_approval_ids.is_empty()
        && receipt.safety.non_repeatable_action.is_none()
        && receipt.final_reason.as_deref()
            == Some("节点更新恢复已熔断：节点发布身份既不是 from_release 也不是目标 release")
}
