//! Durable cancellation saga for active CLI handles and surviving sidecars.

use std::{sync::Arc, time::Duration};

use anyhow::{bail, Result};
use homecli_proto::CancelRequestAudit;
use tracing::{info, warn};

use crate::{
    node_agent_active_task_registry::ActiveCliPromptRegistry,
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry},
    node_agent_local_task_store::LocalTaskStore,
    node_agent_task_journal::{
        CancelIntentRecord, CancelIntentTarget, PersistCancelIntentOutcome, TaskJournal,
    },
    NodeRuntime,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CancelDispatchOutcome {
    Dispatched {
        action_id: String,
        target_kind: String,
        target_id: String,
    },
    AlreadyCommitted {
        action_id: String,
    },
    Pending {
        action_id: String,
    },
    Terminal {
        status: String,
    },
    NotFound,
}

impl CancelDispatchOutcome {
    pub(crate) fn accepted(&self) -> bool {
        matches!(
            self,
            Self::Dispatched { .. } | Self::AlreadyCommitted { .. }
        )
    }
}

pub(crate) async fn request_cancel(
    active: &ActiveCliPromptRegistry,
    sidecars: &CliSidecarRegistry,
    journal: &TaskJournal,
    local_tasks: &LocalTaskStore,
    task_id: &str,
    audit: &CancelRequestAudit,
) -> Result<CancelDispatchOutcome> {
    request_cancel_inner(
        active,
        sidecars,
        journal,
        local_tasks,
        task_id,
        audit,
        false,
    )
    .await
}

async fn request_cancel_inner(
    active: &ActiveCliPromptRegistry,
    sidecars: &CliSidecarRegistry,
    journal: &TaskJournal,
    local_tasks: &LocalTaskStore,
    task_id: &str,
    audit: &CancelRequestAudit,
    stop_after_intent: bool,
) -> Result<CancelDispatchOutcome> {
    let task_id = task_id.trim();
    let active_target = active.cancel_target(task_id).await;
    let sidecar_target = sidecars
        .session_for_task(task_id)?
        .filter(|session| session.can_cancel_at(now_ms()));

    if active_target.is_none() && sidecar_target.is_none() {
        let snapshot = journal.snapshot(task_id, 0, 1)?;
        let Some(record) = snapshot.record else {
            return Ok(CancelDispatchOutcome::NotFound);
        };
        if is_completed(&record.status) {
            return Ok(CancelDispatchOutcome::Terminal {
                status: record.status,
            });
        }
        let Some(intent) = record.cancel_intent else {
            return Ok(CancelDispatchOutcome::NotFound);
        };
        return reconcile_intent(active, sidecars, journal, local_tasks, intent).await;
    }

    let target = CancelIntentTarget {
        run_handle_id: active_target
            .as_ref()
            .map(|target| target.run_handle_id.clone()),
        active_started_at_ms: active_target.as_ref().map(|target| target.started_at_ms),
        sidecar_session_id: sidecar_target
            .as_ref()
            .map(|session| session.session_id.clone()),
    };
    match journal.record_cancel_intent(task_id, target, audit)? {
        PersistCancelIntentOutcome::Pending(intent) => {
            align_local_task(local_tasks, journal, &intent)?;
            if stop_after_intent {
                return Ok(CancelDispatchOutcome::Pending {
                    action_id: intent.action_id,
                });
            }
            reconcile_intent(active, sidecars, journal, local_tasks, intent).await
        }
        PersistCancelIntentOutcome::Committed(intent) => {
            align_local_task(local_tasks, journal, &intent)?;
            Ok(CancelDispatchOutcome::AlreadyCommitted {
                action_id: intent.action_id,
            })
        }
        PersistCancelIntentOutcome::Terminal(status) => {
            Ok(CancelDispatchOutcome::Terminal { status })
        }
        PersistCancelIntentOutcome::Missing => Ok(CancelDispatchOutcome::NotFound),
    }
}

pub(crate) async fn reconcile_intent(
    active: &ActiveCliPromptRegistry,
    sidecars: &CliSidecarRegistry,
    journal: &TaskJournal,
    local_tasks: &LocalTaskStore,
    intent: CancelIntentRecord,
) -> Result<CancelDispatchOutcome> {
    let intent = match journal.cancel_intent_for_reconcile(&intent.task_id, &intent.action_id)? {
        PersistCancelIntentOutcome::Pending(intent) => intent,
        PersistCancelIntentOutcome::Committed(intent) => {
            align_local_task(local_tasks, journal, &intent)?;
            return Ok(CancelDispatchOutcome::AlreadyCommitted {
                action_id: intent.action_id,
            });
        }
        PersistCancelIntentOutcome::Terminal(status) => {
            return Ok(CancelDispatchOutcome::Terminal { status });
        }
        PersistCancelIntentOutcome::Missing => return Ok(CancelDispatchOutcome::NotFound),
    };
    if let Some(status) = align_local_task(local_tasks, journal, &intent)? {
        return Ok(CancelDispatchOutcome::Terminal { status });
    }

    if let Some(expected_handle) = intent.run_handle_id.as_deref() {
        if let Some(target) = active.cancel_target(&intent.task_id).await {
            let identity_matches = target.run_handle_id == expected_handle
                && intent
                    .active_started_at_ms
                    .is_none_or(|started| target.started_at_ms == started);
            if identity_matches && target.cancel_tx.send(true).is_ok() {
                return commit_dispatch(journal, &intent, "active_watch", &target.run_handle_id);
            }
        }
    }

    if let Some(session_id) = intent.sidecar_session_id.as_deref() {
        if sidecars.record_cancel_command_for_session(
            &intent.task_id,
            session_id,
            &intent.action_id,
            &intent.audit,
        )? {
            return commit_dispatch(journal, &intent, "sidecar_mailbox", session_id);
        }
    }

    Ok(CancelDispatchOutcome::Pending {
        action_id: intent.action_id,
    })
}

fn align_local_task(
    local_tasks: &LocalTaskStore,
    journal: &TaskJournal,
    intent: &CancelIntentRecord,
) -> Result<Option<String>> {
    let Some(record) = local_tasks.get(&intent.task_id)? else {
        return Ok(None);
    };
    if is_completed(&record.status) {
        if journal
            .snapshot(&intent.task_id, 0, 1)?
            .record
            .is_some_and(|record| !is_completed(&record.status))
        {
            journal.record_finished_with_outcome(
                &intent.task_id,
                &record.status,
                record.error.as_deref(),
            )?;
        }
        return Ok(Some(record.status));
    }
    if record.status != "cancel_requested" {
        local_tasks.mark_cancel_requested(&intent.task_id)?;
        let refreshed = local_tasks.get(&intent.task_id)?;
        if let Some(refreshed) = refreshed {
            if is_completed(&refreshed.status) {
                return Ok(Some(refreshed.status));
            }
            if refreshed.status != "cancel_requested" {
                bail!(
                    "local task {} cannot align cancel intent from status {}",
                    intent.task_id,
                    refreshed.status
                );
            }
        }
    }
    Ok(None)
}

fn commit_dispatch(
    journal: &TaskJournal,
    intent: &CancelIntentRecord,
    target_kind: &str,
    target_id: &str,
) -> Result<CancelDispatchOutcome> {
    if journal.commit_cancel_side_effect(
        &intent.task_id,
        &intent.action_id,
        target_kind,
        target_id,
    )? {
        return Ok(CancelDispatchOutcome::Dispatched {
            action_id: intent.action_id.clone(),
            target_kind: target_kind.to_string(),
            target_id: target_id.to_string(),
        });
    }
    match journal.cancel_intent_for_reconcile(&intent.task_id, &intent.action_id)? {
        PersistCancelIntentOutcome::Committed(intent) => {
            Ok(CancelDispatchOutcome::AlreadyCommitted {
                action_id: intent.action_id,
            })
        }
        PersistCancelIntentOutcome::Terminal(status) => {
            Ok(CancelDispatchOutcome::Terminal { status })
        }
        PersistCancelIntentOutcome::Pending(intent) => Ok(CancelDispatchOutcome::Pending {
            action_id: intent.action_id,
        }),
        PersistCancelIntentOutcome::Missing => Ok(CancelDispatchOutcome::NotFound),
    }
}

fn is_completed(status: &str) -> bool {
    matches!(
        status,
        "done" | "failed" | "canceled" | "cancelled" | "finished"
    )
}

pub(crate) async fn reconcile_runtime(runtime: &NodeRuntime) -> Result<usize> {
    let intents = runtime.task_journal.pending_cancel_intents()?;
    let mut dispatched = 0;
    for intent in intents {
        match reconcile_intent(
            &runtime.active_cli_prompts,
            &runtime.cli_sidecars,
            &runtime.task_journal,
            &runtime.local_tasks,
            intent,
        )
        .await
        {
            Ok(CancelDispatchOutcome::Dispatched { action_id, .. }) => {
                dispatched += 1;
                info!(%action_id, "replayed durable cancel intent");
            }
            Ok(_) => {}
            Err(error) => warn!(%error, "durable cancel intent reconcile failed"),
        }
    }
    Ok(dispatched)
}

pub(crate) fn spawn_reconciler(runtime: Arc<NodeRuntime>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if let Err(error) = reconcile_runtime(&runtime).await {
                warn!(%error, "periodic durable cancel reconcile failed");
            }
        }
    });
}

#[cfg(test)]
pub(crate) async fn request_cancel_crash_after_intent(
    active: &ActiveCliPromptRegistry,
    sidecars: &CliSidecarRegistry,
    journal: &TaskJournal,
    local_tasks: &LocalTaskStore,
    task_id: &str,
    audit: &CancelRequestAudit,
) -> Result<CancelDispatchOutcome> {
    request_cancel_inner(active, sidecars, journal, local_tasks, task_id, audit, true).await
}

#[cfg(test)]
#[path = "node_agent_cancel_saga_tests.rs"]
mod tests;
