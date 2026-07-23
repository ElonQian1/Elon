//! Blocking projections used by async orphan-reconciliation passes.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};

use crate::{
    node_agent_local_task_supervision::SupervisionContract, node_agent_task_journal::TaskJournal,
};

pub(super) async fn contracts(
    journal: &TaskJournal,
    task_ids: &HashSet<String>,
) -> Result<HashMap<String, Option<SupervisionContract>>> {
    let journal = journal.clone();
    let task_ids = task_ids.clone();
    tokio::task::spawn_blocking(move || {
        crate::node_agent_local_task_supervision::load_supervision_contracts(&journal, &task_ids)
    })
    .await
    .context("orphan supervision contract scan task failed")?
}
