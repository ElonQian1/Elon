use anyhow::Result;

use crate::node_agent_runtime::NodeRuntime;

pub(crate) fn has_finished_success(runtime: &NodeRuntime, task_id: &str) -> Result<bool> {
    Ok(runtime
        .task_journal
        .record(task_id)?
        .is_some_and(|record| record.status == "finished" && record.phase == "done"))
}
