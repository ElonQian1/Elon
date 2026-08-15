use anyhow::{bail, Result};

use super::{super::*, support};

pub(crate) fn validate_task_production_reconcile_poll(
    value: &ExternalPoolAdapterTaskReconcilePollEnvelope,
) -> Result<()> {
    support::metadata(
        &value.schema,
        TASK_PRODUCTION_RECONCILE_POLL_SCHEMA,
        &value.reconcile_poll_id,
        &value.reconcile_poll_digest,
        &value.canonicalization,
        &value.digest_algorithm,
    )?;
    let poll = &value.poll;
    support::poll_lineage(&poll.lineage)?;
    support::identifier(&poll.uncertain_exchange_attempt_id)?;
    support::digest(&poll.uncertain_exchange_attempt_digest)?;
    command(&poll.command)?;
    support::remote(&poll.remote)?;
    if poll.remote.executor_binding_digest != poll.command.executor_binding_digest {
        bail!("task production reconcile remote lane is not exact")
    }
    match poll.authenticated_subject_sha256.as_deref() {
        Some(authenticated_subject_sha256) => {
            support::digest(authenticated_subject_sha256)?;
            let subject = ExternalPoolAdapterTaskAuthenticatedRemoteSubject {
                remote: poll.remote.clone(),
            };
            if canonical_task_production_remote_subject_json_and_sha256(&subject)?.1
                != authenticated_subject_sha256
            {
                bail!("task production reconcile remote subject was not authenticated")
            }
        }
        None if poll.remote.remote_execution_id.is_some()
            || poll.remote.remote_execution_state != "unknown" =>
        {
            bail!("task production unauthenticated reconcile subject is not remote-unknown")
        }
        None => {}
    }
    support::digest(&poll.request_digest)?;
    window(&poll.not_before, &poll.created_at, &poll.not_after)?;
    support::boundary(&poll.boundary)?;
    if canonical_task_production_reconcile_poll_json_and_digest(value)?.1
        != value.reconcile_poll_digest
    {
        bail!("task production reconcile poll digest is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_production_event_poll(
    value: &ExternalPoolAdapterTaskEventPollEnvelope,
) -> Result<()> {
    support::metadata(
        &value.schema,
        TASK_PRODUCTION_EVENT_POLL_SCHEMA,
        &value.event_poll_id,
        &value.event_poll_digest,
        &value.canonicalization,
        &value.digest_algorithm,
    )?;
    let poll = &value.poll;
    support::poll_lineage(&poll.lineage)?;
    support::identifier(&poll.source_exchange_receipt_id)?;
    support::digest(&poll.source_exchange_receipt_digest)?;
    command(&poll.command)?;
    support::remote(&poll.remote)?;
    if poll.remote.executor_binding_digest != poll.command.executor_binding_digest {
        bail!("task production event remote lane is not exact")
    }
    if poll.remote.remote_execution_id.is_none()
        || !matches!(
            poll.remote.remote_execution_state.as_str(),
            "committed" | "running"
        )
    {
        bail!("task production event poll lacks committed remote identity")
    }
    support::digest(&poll.authenticated_subject_sha256)?;
    let subject = ExternalPoolAdapterTaskAuthenticatedRemoteSubject {
        remote: poll.remote.clone(),
    };
    if canonical_task_production_remote_subject_json_and_sha256(&subject)?.1
        != poll.authenticated_subject_sha256
    {
        bail!("task production event poll remote subject was not authenticated")
    }
    event_cursor(&poll.requested_cursor)?;
    support::digest(&poll.request_digest)?;
    window(&poll.not_before, &poll.created_at, &poll.not_after)?;
    support::boundary(&poll.boundary)?;
    if canonical_task_production_event_poll_json_and_digest(value)?.1 != value.event_poll_digest {
        bail!("task production event poll digest is not exact")
    }
    Ok(())
}

pub(super) fn event_cursor(value: &ExternalPoolAdapterTaskEventCursor) -> Result<()> {
    if value.remote_sequence > TASK_PRODUCTION_MAX_SAFE_INTEGER {
        bail!("task production event cursor sequence is invalid")
    }
    match (value.remote_sequence, value.previous_event_root.as_deref()) {
        (0, None) => {}
        (0, Some(_)) | (_, None) => bail!("task production event cursor root is incomplete"),
        (_, Some(previous_event_root)) => support::digest(previous_event_root)?,
    }
    support::digest(&value.cursor_digest)?;
    if task_production_event_cursor_digest(
        value.remote_sequence,
        value.previous_event_root.as_deref(),
    )? != value.cursor_digest
    {
        bail!("task production event cursor digest is not exact")
    }
    Ok(())
}

fn command(value: &ExternalPoolAdapterTaskPollCommandBinding) -> Result<()> {
    for id in [
        &value.command_id,
        &value.outbox_id,
        &value.send_attempt_id,
        &value.route_authorization_id,
    ] {
        support::identifier(id)?;
    }
    for digest_value in [
        &value.command_digest,
        &value.outbox_digest,
        &value.send_attempt_digest,
        &value.route_authorization_digest,
        &value.executor_binding_digest,
        &value.fence_digest,
    ] {
        support::digest(digest_value)?;
    }
    if value.fencing_generation == 0 || value.fencing_generation > TASK_PRODUCTION_MAX_SAFE_INTEGER
    {
        bail!("task production poll fencing generation is invalid")
    }
    Ok(())
}

fn window(not_before: &str, created_at: &str, not_after: &str) -> Result<()> {
    let not_before = support::canonical_nanos(not_before)?;
    let created_at = support::canonical_nanos(created_at)?;
    let not_after = support::canonical_nanos(not_after)?;
    if not_before > created_at || created_at > not_after || not_before >= not_after {
        bail!("task production poll window is invalid")
    }
    Ok(())
}
