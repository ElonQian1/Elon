//! One-pass supervision contract projection for bounded recovery audits.

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
};

use anyhow::Context;
use serde_json::Value;

use super::SupervisionContract;
use crate::{
    node_agent_task_journal::TaskJournal, node_agent_task_journal_lock::with_task_journal_io_lock,
};

pub(crate) fn load_supervision_contract(
    journal: &TaskJournal,
    task_id: &str,
) -> anyhow::Result<Option<SupervisionContract>> {
    Ok(super::load_supervision_state(journal, task_id)?.contract)
}

/// Loads the latest supervision contract for several tasks with one journal
/// pass. Invalid/missing contracts remain `None`, matching the single-task
/// projection while avoiding one full append-only scan per recovery candidate.
pub(crate) fn load_supervision_contracts(
    journal: &TaskJournal,
    task_ids: &HashSet<String>,
) -> anyhow::Result<HashMap<String, Option<SupervisionContract>>> {
    with_task_journal_io_lock(|| {
        let mut contracts = task_ids
            .iter()
            .cloned()
            .map(|task_id| (task_id, None))
            .collect::<HashMap<_, _>>();
        let path = journal.events_path();
        if !path.exists() || task_ids.is_empty() {
            return Ok(contracts);
        }
        let file = File::open(&path).with_context(|| format!("打开 {:?}", path))?;
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line.with_context(|| format!("读取 {:?}", path))?;
            let event: Value = match serde_json::from_str(&line) {
                Ok(event) => event,
                Err(error) => {
                    tracing::warn!(
                        path = %path.display(),
                        seq = index + 1,
                        %error,
                        "skipping corrupt supervision journal event line"
                    );
                    continue;
                }
            };
            if event.get("type").and_then(Value::as_str) != Some("supervision_contract") {
                continue;
            }
            let Some(task_id) = event.get("req_id").and_then(Value::as_str) else {
                continue;
            };
            let Some(contract) = contracts.get_mut(task_id) else {
                continue;
            };
            *contract = event
                .get("payload")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok());
        }
        Ok(contracts)
    })
}
