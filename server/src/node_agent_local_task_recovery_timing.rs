//! User-facing continuation timing that separates handoff from resumed work.

use anyhow::Result;
use serde::Serialize;

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{SupervisionContract, SupervisionState},
    NodeRuntime,
};

const HANDOFF_TARGET_MS: i64 = 8 * 60 * 1_000;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct RecoveryTiming {
    mode: &'static str,
    parent_task_id: String,
    handoff_ms: Option<i64>,
    resumed_work_ms: i64,
    total_since_parent_finished_ms: Option<i64>,
    handoff_target_ms: i64,
    handoff_within_target: Option<bool>,
}

pub(crate) fn build(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    contract: Option<&SupervisionContract>,
) -> Result<Option<RecoveryTiming>> {
    let Some(contract) = contract.filter(|contract| contract.task_role == "resume_original") else {
        return Ok(None);
    };
    let Some(parent_task_id) = contract.parent_task_id.as_deref() else {
        return Ok(None);
    };
    let parent = runtime
        .local_tasks
        .get_for_owner(&task.owner_user_id, parent_task_id)?;
    let mode = if crate::node_agent_local_task_contract_revision::task_has_revision(
        runtime,
        &task.task_id,
    )? {
        "supersede"
    } else {
        "resume"
    };
    Ok(Some(calculate(
        mode,
        parent_task_id,
        parent.as_ref().and_then(|parent| parent.finished_at_ms),
        task.started_at_ms,
        task.finished_at_ms,
        now_ms(),
    )))
}

pub(crate) fn build_best_effort(
    runtime: &NodeRuntime,
    task: &LocalTaskRecord,
    supervision: &SupervisionState,
) -> Option<RecoveryTiming> {
    match build(runtime, task, supervision.contract()) {
        Ok(timing) => timing,
        Err(error) => {
            tracing::warn!(task_id = %task.task_id, %error, "recovery timing unavailable");
            None
        }
    }
}

fn calculate(
    mode: &'static str,
    parent_task_id: &str,
    parent_finished_at_ms: Option<i64>,
    started_at_ms: i64,
    finished_at_ms: Option<i64>,
    now_ms: i64,
) -> RecoveryTiming {
    let ended_at_ms = finished_at_ms.unwrap_or(now_ms).max(started_at_ms);
    let handoff_ms = parent_finished_at_ms
        .filter(|parent_finished| *parent_finished <= started_at_ms)
        .map(|parent_finished| started_at_ms.saturating_sub(parent_finished));
    let total_since_parent_finished_ms = parent_finished_at_ms
        .filter(|parent_finished| *parent_finished <= ended_at_ms)
        .map(|parent_finished| ended_at_ms.saturating_sub(parent_finished));
    RecoveryTiming {
        mode,
        parent_task_id: parent_task_id.to_string(),
        handoff_ms,
        resumed_work_ms: ended_at_ms.saturating_sub(started_at_ms),
        total_since_parent_finished_ms,
        handoff_target_ms: HANDOFF_TARGET_MS,
        handoff_within_target: handoff_ms.map(|value| value <= HANDOFF_TARGET_MS),
    }
}

fn now_ms() -> i64 {
    crate::node_agent_cli_sidecar::now_ms().min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_handoff_from_resumed_work() {
        let timing = calculate(
            "resume",
            "parent",
            Some(1_000),
            91_000,
            Some(391_000),
            999_000,
        );
        assert_eq!(timing.handoff_ms, Some(90_000));
        assert_eq!(timing.resumed_work_ms, 300_000);
        assert_eq!(timing.total_since_parent_finished_ms, Some(390_000));
        assert_eq!(timing.handoff_within_target, Some(true));
    }

    #[test]
    fn negative_or_missing_parent_gap_is_not_invented() {
        let timing = calculate("supersede", "parent", Some(10_000), 5_000, None, 15_000);
        assert_eq!(timing.handoff_ms, None);
        assert_eq!(timing.resumed_work_ms, 10_000);
        assert_eq!(timing.total_since_parent_finished_ms, Some(5_000));
        assert_eq!(timing.handoff_within_target, None);
    }
}
