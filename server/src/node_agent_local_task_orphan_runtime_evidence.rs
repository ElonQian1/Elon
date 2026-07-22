use anyhow::Result;

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
