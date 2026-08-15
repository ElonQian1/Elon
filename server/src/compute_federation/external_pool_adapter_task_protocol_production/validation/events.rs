use anyhow::{bail, Result};

use super::{super::*, polls::event_cursor, support};

pub(crate) fn validate_task_production_event_batch(
    value: &ExternalPoolAdapterTaskEventBatchEnvelope,
) -> Result<()> {
    support::metadata(
        &value.schema,
        TASK_PRODUCTION_EVENT_BATCH_SCHEMA,
        &value.event_batch_id,
        &value.event_batch_digest,
        &value.canonicalization,
        &value.digest_algorithm,
    )?;
    let batch = &value.batch;
    for id in [&batch.event_poll_id, &batch.exchange_receipt_id] {
        support::identifier(id)?;
    }
    for digest_value in [&batch.event_poll_digest, &batch.exchange_receipt_digest] {
        support::digest(digest_value)?;
    }
    support::optional_pair(
        batch.predecessor_event_batch_id.as_deref(),
        batch.predecessor_event_batch_digest.as_deref(),
    )?;
    support::remote(&batch.remote)?;
    if batch.remote.remote_execution_id.is_none() {
        bail!("task production event batch lacks committed remote identity")
    }
    if !matches!(
        batch.remote.remote_execution_state.as_str(),
        "committed" | "running" | "terminal_after_run"
    ) {
        bail!("task production event batch remote state is invalid")
    }
    support::digest(&batch.authenticated_observation_sha256)?;
    event_cursor(&batch.cursor_before)?;
    event_cursor(&batch.cursor_after)?;
    support::digest(&batch.batch_root)?;
    support::digest(&batch.event_inventory_digest)?;
    if !((batch.event_count == 0 && batch.replay_classification == "empty")
        || (batch.event_count > 0 && batch.replay_classification == "new"))
    {
        bail!("task production event replay classification is invalid")
    }
    if batch.event_count > TASK_PRODUCTION_MAX_EVENTS_PER_BATCH
        || batch.event_roots.len() as u64 != batch.event_count
    {
        bail!("task production event batch count is invalid")
    }
    for root in &batch.event_roots {
        support::digest(root)?;
    }
    let expected_after = batch
        .cursor_before
        .remote_sequence
        .checked_add(batch.event_count)
        .ok_or_else(|| anyhow::anyhow!("task production event cursor overflow"))?;
    let expected_after_root = batch
        .event_roots
        .last()
        .cloned()
        .or_else(|| batch.cursor_before.previous_event_root.clone());
    if batch.cursor_after.remote_sequence != expected_after
        || batch.cursor_after.previous_event_root != expected_after_root
        || task_production_event_inventory_digest(&batch.event_roots)?
            != batch.event_inventory_digest
        || task_production_event_batch_root(batch)? != batch.batch_root
    {
        bail!("task production event batch roots are not contiguous")
    }
    let observation = task_production_authenticated_event_observation(batch);
    if canonical_task_production_authenticated_event_observation_json_and_sha256(&observation)?.1
        != batch.authenticated_observation_sha256
    {
        bail!("task production event batch observation was not authenticated")
    }
    match (
        batch.predecessor_event_batch_id.as_deref(),
        batch.previous_batch_root.as_deref(),
    ) {
        (None, None) => {}
        (Some(_), Some(root)) => support::digest(root)?,
        _ => bail!("task production event batch predecessor root is partial"),
    }
    let authenticated_at = support::canonical_nanos(&batch.authenticated_at)?;
    let received_at = support::canonical_nanos(&batch.received_at)?;
    let recorded_at = support::canonical_nanos(&batch.recorded_at)?;
    if authenticated_at > received_at || received_at > recorded_at {
        bail!("task production event batch timestamps are out of order")
    }
    support::boundary(&batch.boundary)?;
    if canonical_task_production_event_batch_json_and_digest(value)?.1 != value.event_batch_digest {
        bail!("task production event batch digest is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_production_event_remote_state_transition(
    before: &str,
    after: &str,
) -> Result<()> {
    if !matches!(
        (before, after),
        ("committed", "committed" | "running" | "terminal_after_run")
            | ("running", "running" | "terminal_after_run")
    ) {
        bail!("task production event remote state transition is not monotonic")
    }
    Ok(())
}

pub(crate) fn validate_task_production_event(
    value: &ExternalPoolAdapterTaskEventEnvelope,
) -> Result<()> {
    support::metadata(
        &value.schema,
        TASK_PRODUCTION_EVENT_SCHEMA,
        &value.event_id,
        &value.event_digest,
        &value.canonicalization,
        &value.digest_algorithm,
    )?;
    let event = &value.event;
    support::identifier(&event.event_batch_id)?;
    support::digest(&event.event_batch_digest)?;
    support::digest(&event.remote_identity_digest)?;
    if event.event_ordinal == 0 || event.event_ordinal > TASK_PRODUCTION_MAX_EVENTS_PER_BATCH {
        bail!("task production event ordinal is invalid")
    }
    support::identifier(&event.remote_event_id)?;
    support::text(&event.event_type, 1, 120)?;
    if event.remote_sequence == 0 || event.remote_sequence > TASK_PRODUCTION_MAX_SAFE_INTEGER {
        bail!("task production remote event sequence is invalid")
    }
    match (event.remote_sequence, event.previous_event_root.as_deref()) {
        (1, None) => {}
        (1, Some(_)) | (_, None) => bail!("task production previous event root is incomplete"),
        (_, Some(root)) => support::digest(root)?,
    }
    support::digest(&event.event_root)?;
    support::digest(&event.canonical_event_digest)?;
    let observed_at = support::canonical_nanos(&event.observed_at)?;
    let recorded_at = support::canonical_nanos(&event.recorded_at)?;
    if observed_at > recorded_at {
        bail!("task production event timestamps are out of order")
    }
    support::boundary(&event.boundary)?;
    if task_production_event_root(event)? != event.event_root
        || canonical_task_production_event_json_and_digest(value)?.1 != value.event_digest
    {
        bail!("task production event digest is not exact")
    }
    Ok(())
}
