//! Live Codex thread projection for local-task HTTP responses.

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_task_journal::{TaskJournal, TaskJournalRecord},
};

pub(crate) fn enrich_list(journal: &TaskJournal, tasks: &mut [LocalTaskRecord]) {
    let task_ids = tasks
        .iter()
        .map(|task| task.task_id.clone())
        .collect::<Vec<_>>();
    let records = match journal.records_for_req_ids(&task_ids) {
        Ok(records) => records,
        Err(error) => {
            tracing::warn!(%error, "读取本机任务 Codex 会话 ID 失败");
            return;
        }
    };
    for task in tasks {
        enrich(task, records.get(&task.task_id));
    }
}

pub(crate) fn enrich(task: &mut LocalTaskRecord, journal: Option<&TaskJournalRecord>) {
    if task.codex_session_id.is_none() {
        task.codex_session_id = journal.and_then(|record| record.codex_session_id.clone());
    }
}
