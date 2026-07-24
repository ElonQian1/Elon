use std::path::Path;

use anyhow::Result;

use crate::NodeRuntime;

pub(super) async fn exact_runtime_protects(
    runtime: &NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    active_workspace: &Path,
    now: u128,
    stale_after_ms: u128,
) -> Result<bool> {
    if runtime
        .active_cli_prompt_views_for_workspace(active_workspace)
        .await
        .into_iter()
        .any(|handle| handle.control_handle_live)
        || runtime
            .active_cli_prompt_view(&task.task_id)
            .await
            .is_some_and(|handle| handle.control_handle_live)
    {
        return Ok(true);
    }
    if let Some(sidecar) = runtime.cli_sidecars.session_for_task(&task.task_id)? {
        if sidecar_record_protects(&sidecar, now)? {
            return Ok(true);
        }
    }
    if let Some(record) = runtime.task_journal.record(&task.task_id)? {
        if journal_record_protects(&record, now, stale_after_ms)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn journal_record_protects(
    record: &crate::node_agent_task_journal::TaskJournalRecord,
    now: u128,
    stale_after_ms: u128,
) -> Result<bool> {
    if !matches!(
        record.status.as_str(),
        "running" | "recovering" | "reattaching" | "cancel_requested"
    ) {
        return Ok(false);
    }
    let heartbeat = record.heartbeat_at_ms.unwrap_or(record.updated_at_ms);
    anyhow::ensure!(heartbeat <= now, "journal heartbeat is in the future");
    if now.saturating_sub(heartbeat) <= stale_after_ms {
        return Ok(true);
    }
    recorded_process_is_live(record)
}

pub(crate) fn recorded_process_is_live(
    record: &crate::node_agent_task_journal::TaskJournalRecord,
) -> Result<bool> {
    let Some(pid) = record.os_pid else {
        return Ok(false);
    };
    let running = crate::node_agent_cli_worker::process_is_running(pid);
    let current_identity = crate::node_agent_cli_worker::process_identity(pid);
    match (
        running,
        record.process_identity.as_deref(),
        current_identity,
    ) {
        (false, _, _) => Ok(false),
        (true, Some(expected), Some(actual)) => Ok(expected == actual),
        (true, _, _) => anyhow::bail!("live journal PID has no verifiable process identity"),
    }
}

pub(super) fn sidecar_record_protects(
    session: &crate::node_agent_cli_sidecar::CliSidecarSessionRecord,
    now: u128,
) -> Result<bool> {
    anyhow::ensure!(
        session.last_seen_at_ms <= now && session.started_at_ms <= now,
        "sidecar heartbeat is in the future"
    );
    if session.is_terminal() {
        return Ok(false);
    }
    if session.protects_startup_reconcile_at(now) {
        return Ok(true);
    }
    for (pid, identity) in [
        (
            session.sidecar_pid,
            session.sidecar_process_identity.as_deref(),
        ),
        (session.child_pid, session.child_process_identity.as_deref()),
    ] {
        let Some(pid) = pid else { continue };
        if !crate::node_agent_cli_worker::process_is_running(pid) {
            continue;
        }
        let current = crate::node_agent_cli_worker::process_identity(pid);
        match (identity, current) {
            (Some(expected), Some(actual)) if expected == actual => return Ok(true),
            (Some(_), Some(_)) => continue,
            _ => anyhow::bail!("live sidecar PID has no verifiable process identity"),
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "node_agent_local_task_orphan_resume_receipt_tests.rs"]
mod resume_receipt_tests;
