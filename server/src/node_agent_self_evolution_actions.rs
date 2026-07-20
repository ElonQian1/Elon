//! Durable action-intent transitions for the self-evolution queue.

use anyhow::{Context, Result};
use homecli_proto::InterruptionSource;
use uuid::Uuid;

use super::{now_ms, PendingSelfEvolutionAction, SelfEvolutionCoordinator};

impl SelfEvolutionCoordinator {
    pub(super) fn request_gate_pauses(&self) -> Result<Vec<(String, String, String, String)>> {
        self.mutate(|state| {
            let Some(reason) = state.gates.blocker().map(str::to_string) else {
                return Ok(Vec::new());
            };
            let mut requests = Vec::new();
            for item in &mut state.items {
                if item.status != "running" {
                    continue;
                }
                let Some(task_id) = item.active_task_id.clone() else {
                    continue;
                };
                if item
                    .pending_action
                    .as_ref()
                    .is_some_and(|intent| intent.action == "pause")
                {
                    requests.push((
                        item.owner_user_id.clone(),
                        item.logical_id.clone(),
                        task_id,
                        reason.clone(),
                    ));
                    continue;
                }
                item.pause_reason = Some(reason.clone());
                item.yield_reason = Some(reason.clone());
                item.pending_action = Some(PendingSelfEvolutionAction {
                    action_id: format!("evolution-action-{}", Uuid::new_v4()),
                    action: "pause".to_string(),
                    note: Some(format!("yield_for_{reason}")),
                    actor: "node_agent".to_string(),
                    source: if reason == "node_update" {
                        "updater_apply".to_string()
                    } else {
                        "supervisor".to_string()
                    },
                    requested_at_ms: now_ms(),
                });
                item.updated_at_ms = now_ms();
                requests.push((
                    item.owner_user_id.clone(),
                    item.logical_id.clone(),
                    task_id,
                    reason.clone(),
                ));
            }
            Ok(requests)
        })
    }

    pub(super) fn begin_action(
        &self,
        owner: &str,
        logical_id: &str,
        action: &str,
        note: Option<String>,
        actor: &str,
        source: &str,
    ) -> Result<(super::SelfEvolutionItem, Option<String>)> {
        self.mutate(|state| {
            let item = state
                .items
                .iter_mut()
                .find(|item| item.owner_user_id == owner && item.logical_id == logical_id)
                .context("self evolution item not found")?;
            if item
                .pending_action
                .as_ref()
                .is_some_and(|pending| pending.action == action)
            {
                return Ok((item.clone(), item.active_task_id.clone()));
            }
            let valid = match action {
                "pause" => matches!(item.status.as_str(), "running" | "starting"),
                "resume" => matches!(item.status.as_str(), "paused" | "failed" | "retry_wait"),
                "approve" | "reject" => item.status == "review_required",
                _ => false,
            };
            if !valid {
                anyhow::bail!("self evolution action is not valid for current state");
            }
            item.pending_action = Some(PendingSelfEvolutionAction {
                action_id: format!("evolution-action-{}", Uuid::new_v4()),
                action: action.to_string(),
                note,
                actor: actor.to_string(),
                source: source.to_string(),
                requested_at_ms: now_ms(),
            });
            item.updated_at_ms = now_ms();
            Ok((item.clone(), item.active_task_id.clone()))
        })
    }

    pub(super) fn commit_action(
        &self,
        owner: &str,
        logical_id: &str,
        action: &str,
    ) -> Result<super::SelfEvolutionItem> {
        self.mutate(|state| {
            let item = state
                .items
                .iter_mut()
                .find(|item| item.owner_user_id == owner && item.logical_id == logical_id)
                .context("self evolution item not found")?;
            let pending = item
                .pending_action
                .take()
                .context("self evolution action has no durable intent")?;
            if pending.action != action {
                anyhow::bail!("self evolution pending action does not match");
            }
            match action {
                "pause" => {
                    item.status = "pause_requested".to_string();
                    item.pause_reason = Some("manual_pause".to_string());
                    item.yield_reason = Some("manual_pause".to_string());
                    item.interruption_source = Some(InterruptionSource::SupervisorIntervention);
                }
                "resume" => {
                    item.status = "queued".to_string();
                    item.pause_reason = None;
                    item.review_verdict = None;
                    item.next_retry_at_ms = None;
                }
                "approve" => {
                    item.active_task_id = None;
                    item.status = "completed".to_string();
                    item.review_verdict = Some("approved".to_string());
                    item.review_note = pending.note;
                }
                "reject" => {
                    item.active_task_id = None;
                    item.status = "paused".to_string();
                    item.review_verdict = Some("changes_requested".to_string());
                    item.review_note = pending.note;
                    item.pause_reason = Some("review_changes_requested".to_string());
                }
                _ => anyhow::bail!("unsupported self evolution action"),
            }
            if matches!(action, "approve" | "reject") {
                item.reviewed_by = Some(pending.actor);
                item.review_source = Some(pending.source);
                item.reviewed_at_ms = Some(now_ms());
            }
            item.updated_at_ms = now_ms();
            Ok(item.clone())
        })
    }
}
