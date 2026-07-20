//! Durable supervision-review saga.
//!
//! The journal intent is persisted before update-recovery and Git lease side
//! effects. Reconciliation replays any intent without a matching commit after
//! a node restart; every side effect is independently idempotent.

use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::{
    load_supervision_state, record_supervision_event, record_update_review_if_present,
    release_accepted_worktree_lease, SupervisionContract, SupervisionReview,
};
use crate::{
    node_agent_local_task_store::LocalTaskRecord, node_agent_task_journal::TaskJournal,
    node_agent_task_journal_lock::with_task_journal_io_lock, NodeRuntime,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ReviewActionIntent {
    action_id: String,
    action: String,
    task_id: String,
    review: SupervisionReview,
    requested_at_ms: u128,
}

pub(super) fn begin_review_action(
    journal: &TaskJournal,
    task_id: &str,
    review: &SupervisionReview,
) -> Result<String> {
    let intent = ReviewActionIntent {
        action_id: format!("supervision-review-{}", Uuid::new_v4()),
        action: "review".to_string(),
        task_id: task_id.to_string(),
        review: review.clone(),
        requested_at_ms: super::now_ms(),
    };
    with_task_journal_io_lock(|| {
        if let Some(action_id) = latest_review_unlocked(journal, task_id)?
            .filter(|current| same_review_identity(current, review))
            .and_then(|current| current.action_id)
        {
            return Ok(action_id);
        }
        if let Some(existing) =
            pending_review_actions_unlocked(journal)?
                .into_iter()
                .find(|current| {
                    current.task_id == task_id && same_review_identity(&current.review, review)
                })
        {
            return Ok(existing.action_id);
        }
        journal.append_event(json!({
            "type": "supervision_action_intent",
            "req_id": task_id,
            "payload": intent,
            "at_ms": super::now_ms(),
        }))?;
        Ok(intent.action_id)
    })
}

fn latest_review_unlocked(
    journal: &TaskJournal,
    task_id: &str,
) -> Result<Option<SupervisionReview>> {
    let path = journal.events_path();
    if !path.exists() {
        return Ok(None);
    }
    let file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
    let mut latest = None;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| format!("read {} line {}", path.display(), index + 1))?;
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if event.get("type").and_then(serde_json::Value::as_str) == Some("supervision_review")
            && event.get("req_id").and_then(serde_json::Value::as_str) == Some(task_id)
        {
            latest = event
                .get("payload")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok());
        }
    }
    Ok(latest)
}

pub(super) fn apply_review_action(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    contract: Option<&SupervisionContract>,
    task_id: &str,
    action_id: &str,
    mut review: SupervisionReview,
) -> Result<()> {
    anyhow::ensure!(task.task_id == task_id, "review saga task identity drifted");
    review.action_id = Some(action_id.to_string());
    let already_recorded = load_supervision_state(&runtime.task_journal, task_id)?
        .review
        .as_ref()
        .is_some_and(|current| current.action_id.as_deref() == Some(action_id));
    if !already_recorded {
        record_supervision_event(
            &runtime.task_journal,
            task_id,
            "supervision_review",
            serde_json::to_value(&review)?,
        )?;
    }
    record_update_review_if_present(runtime, task_id, &review)?;
    if review.verdict == "accepted" {
        release_accepted_worktree_lease(runtime, task, contract, task_id)?;
    }
    commit_review_action(&runtime.task_journal, task_id, action_id)
}

pub(super) fn reconcile_pending_review_actions(runtime: &NodeRuntime) -> Result<()> {
    for intent in pending_review_actions(&runtime.task_journal)? {
        let task = runtime
            .local_tasks
            .get(&intent.task_id)?
            .with_context(|| format!("pending review task {} was not found", intent.task_id))?;
        let contract = load_supervision_state(&runtime.task_journal, &intent.task_id)?.contract;
        apply_review_action(
            runtime,
            &task,
            contract.as_ref(),
            &intent.task_id,
            &intent.action_id,
            intent.review,
        )?;
    }
    Ok(())
}

fn commit_review_action(journal: &TaskJournal, task_id: &str, action_id: &str) -> Result<()> {
    with_task_journal_io_lock(|| {
        if !pending_review_actions_unlocked(journal)?
            .iter()
            .any(|intent| intent.action_id == action_id)
        {
            return Ok(());
        }
        journal.append_event(json!({
            "type": "supervision_action_committed",
            "req_id": task_id,
            "payload": {"action_id": action_id, "action": "review"},
            "at_ms": super::now_ms(),
        }))
    })
}

fn pending_review_actions(journal: &TaskJournal) -> Result<Vec<ReviewActionIntent>> {
    with_task_journal_io_lock(|| pending_review_actions_unlocked(journal))
}

fn pending_review_actions_unlocked(journal: &TaskJournal) -> Result<Vec<ReviewActionIntent>> {
    let path = journal.events_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(&path).with_context(|| format!("open {}", path.display()))?;
    let mut intents = BTreeMap::new();
    let mut committed = HashSet::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| format!("read {} line {}", path.display(), index + 1))?;
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        match event.get("type").and_then(serde_json::Value::as_str) {
            Some("supervision_action_intent") => {
                if let Some(intent) = event
                    .get("payload")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<ReviewActionIntent>(value).ok())
                    .filter(|intent| intent.action == "review")
                {
                    intents.insert(intent.action_id.clone(), intent);
                }
            }
            Some("supervision_action_committed") => {
                if let Some(action_id) = event
                    .get("payload")
                    .and_then(|value| value.get("action_id"))
                    .and_then(serde_json::Value::as_str)
                {
                    committed.insert(action_id.to_string());
                }
            }
            _ => {}
        }
    }
    Ok(intents
        .into_iter()
        .filter_map(|(id, intent)| (!committed.contains(&id)).then_some(intent))
        .collect())
}

fn same_review_identity(left: &SupervisionReview, right: &SupervisionReview) -> bool {
    left.verdict == right.verdict
        && left.summary == right.summary
        && left.improvements == right.improvements
        && left.reviewed_by == right.reviewed_by
        && left.review_source == right.review_source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_intent_survives_restart_and_commit_is_idempotent() {
        let dir =
            std::env::temp_dir().join(format!("elon-review-saga-{}", Uuid::new_v4().simple()));
        let journal = TaskJournal::new(&dir);
        let review = SupervisionReview {
            action_id: None,
            protocol: super::super::SUPERVISION_PROTOCOL.to_string(),
            verdict: "accepted".to_string(),
            summary: "verified".to_string(),
            improvements: Vec::new(),
            reviewed_by: "codex_desktop:owner-a".to_string(),
            review_source: "codex_desktop_helper".to_string(),
            reviewed_at_ms: 7,
        };
        let action_id = begin_review_action(&journal, "task-a", &review).unwrap();
        let restarted = TaskJournal::new(&dir);
        let pending = pending_review_actions(&restarted).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action_id, action_id);

        commit_review_action(&restarted, "task-a", &action_id).unwrap();
        commit_review_action(&restarted, "task-a", &action_id).unwrap();
        assert!(pending_review_actions(&restarted).unwrap().is_empty());

        let mut recorded = review.clone();
        recorded.action_id = Some(action_id.clone());
        record_supervision_event(
            &restarted,
            "task-a",
            "supervision_review",
            serde_json::to_value(recorded).unwrap(),
        )
        .unwrap();
        assert_eq!(
            begin_review_action(&restarted, "task-a", &review).unwrap(),
            action_id
        );
        assert!(pending_review_actions(&restarted).unwrap().is_empty());
    }
}
