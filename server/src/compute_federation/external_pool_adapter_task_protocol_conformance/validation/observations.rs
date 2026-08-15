use std::collections::BTreeSet;

use anyhow::{bail, Result};

use super::{super::*, support::*};

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_observations(
    exchanges: &[TaskProtocolConformanceExchangeObservation],
    capabilities: &[TaskProtocolConformanceCapabilityObservation],
    cleanup: &TaskProtocolConformanceCleanupEvidence,
    task_protocol_profile_digest: &str,
    fixture_catalog_digest: &str,
    delivery_inventory_digest: &str,
    exchange_inventory_digest: &str,
    task_observation_root: &str,
) -> Result<()> {
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    if exchanges.len() != TASK_PROTOCOL_CONFORMANCE_EXCHANGE_COUNT
        || capabilities.len() != TASK_PROTOCOL_CONFORMANCE_CAPABILITY_COUNT
        || !cleanup.authenticated_shutdown_completed
        || !cleanup.pidfd_reaped
        || !cleanup.cgroup_cleaned
        || !cleanup.scratch_cleaned
    {
        bail!("task-protocol conformance evidence inventory is incomplete")
    }
    for (index, (actual, expected)) in exchanges
        .iter()
        .zip(fixture.catalog.exchanges.iter())
        .enumerate()
    {
        validate_exchange(actual, expected, index)?;
    }
    validate_exchange_continuity(exchanges)?;
    if task_protocol_conformance_delivery_inventory_digest(exchanges)? != delivery_inventory_digest
        || task_protocol_conformance_exchange_inventory_digest(exchanges)?
            != exchange_inventory_digest
    {
        bail!("task-protocol conformance delivery or exchange inventory drifted")
    }
    for (index, ((actual, capability_id), ordinals)) in capabilities
        .iter()
        .zip(fixture.catalog.capability_order.iter())
        .zip(fixture.catalog.capability_exchange_ordinals.iter())
        .enumerate()
    {
        let capability_id = capability_id.as_str();
        let expected_test_case =
            format!("external_pool_adapter_task_protocol_conformance_{capability_id}_v1");
        let expected_fixture_digest = task_protocol_conformance_capability_fixture_digest(
            task_protocol_profile_digest,
            fixture_catalog_digest,
            capability_id,
            1,
            ordinals,
        )?;
        let selected: Vec<TaskProtocolConformanceExchangeObservation> = ordinals
            .iter()
            .map(|ordinal| {
                let zero_based = ordinal
                    .checked_sub(1)
                    .ok_or_else(|| anyhow::anyhow!("task-protocol conformance ordinal is zero"))?;
                let index = usize::try_from(zero_based)?;
                exchanges.get(index).cloned().ok_or_else(|| {
                    anyhow::anyhow!("task-protocol conformance ordinal is outside evidence")
                })
            })
            .collect::<Result<_>>()?;
        let expected_exchange_inventory =
            task_protocol_conformance_exchange_inventory_digest(&selected)?;
        if actual.capability_id != capability_id
            || actual.capability_revision != 1
            || actual.status != "passed_server_run"
            || actual.test_case_id != expected_test_case
            || actual.fixture_digest != expected_fixture_digest
            || &actual.exchange_ordinals != ordinals
            || actual.exchange_inventory_digest != expected_exchange_inventory
            || actual.assertion_inventory_digest
                != task_protocol_conformance_capability_assertion_inventory_digest(
                    &actual.capability_id,
                    actual.capability_revision,
                    &actual.status,
                    &actual.test_case_id,
                    &actual.fixture_digest,
                    &actual.exchange_ordinals,
                    &actual.exchange_inventory_digest,
                )?
        {
            bail!("task-protocol conformance capability observation {index} is not exact")
        }
    }
    if task_protocol_conformance_task_observation_root(exchanges, capabilities, cleanup)?
        != task_observation_root
    {
        bail!("task-protocol conformance task observation root is not exact")
    }
    Ok(())
}

fn validate_exchange_continuity(
    exchanges: &[TaskProtocolConformanceExchangeObservation],
) -> Result<()> {
    let exchange = |ordinal: usize| {
        let index = ordinal
            .checked_sub(1)
            .ok_or_else(|| anyhow::anyhow!("task-protocol conformance ordinal is zero"))?;
        exchanges.get(index).ok_or_else(|| {
            anyhow::anyhow!("task-protocol conformance continuity exchange is absent")
        })
    };
    let one = exchange(1)?;
    let two = exchange(2)?;
    let three = exchange(3)?;
    let four = exchange(4)?;
    let five = exchange(5)?;
    let six = exchange(6)?;
    let seven = exchange(7)?;
    let eight = exchange(8)?;
    let a_reference = &one.remote_reference_digest;
    let b_reference = &six.remote_reference_digest;
    let expected_uncertainty_marker =
        task_protocol_conformance_commit_uncertainty_marker_digest(three)?;
    if [two, three, four, five]
        .into_iter()
        .any(|item| &item.remote_reference_digest != a_reference)
        || [seven, eight]
            .into_iter()
            .any(|item| &item.remote_reference_digest != b_reference)
        || a_reference == b_reference
        || [two, three, four, five]
            .into_iter()
            .any(|item| item.command_digest != one.command_digest)
        || [seven, eight]
            .into_iter()
            .any(|item| item.command_digest != six.command_digest)
        || one.command_digest == six.command_digest
        || two.outbox_operation_digest != three.outbox_operation_digest
        || two.route_authorization_digest != three.route_authorization_digest
        || two.synthetic_executor_digest != three.synthetic_executor_digest
        || two.fence_digest != three.fence_digest
        || two.request_digest != three.request_digest
        || two.upstream_request_bytes != three.upstream_request_bytes
        || two.upstream_request_sha256 != three.upstream_request_sha256
        || two.upstream_response_bytes != three.upstream_response_bytes
        || two.upstream_response_sha256 != three.upstream_response_sha256
        || three.commit_uncertainty_marker_digest.is_none()
        || three.commit_uncertainty_marker_digest != four.commit_uncertainty_marker_digest
        || three.commit_uncertainty_marker_digest.as_deref()
            != Some(expected_uncertainty_marker.as_str())
        || three.remote_sequence != four.remote_sequence
        || three.oracle_start_count_after != four.oracle_start_count_before
        || four.oracle_start_count_before != four.oracle_start_count_after
        || two.outbox_operation_digest == four.outbox_operation_digest
        || two.request_digest == four.request_digest
        || exchanges
            .iter()
            .skip(3)
            .any(|item| item.operation_kind == "idempotent_commit")
    {
        bail!("task-protocol conformance exact replay or reconcile recovery continuity drifted")
    }
    let route_authorization = &one.route_authorization_digest;
    let synthetic_executor = &one.synthetic_executor_digest;
    let fence = &one.fence_digest;
    if exchanges.iter().skip(1).any(|item| {
        &item.route_authorization_digest != route_authorization
            || &item.synthetic_executor_digest != synthetic_executor
            || &item.fence_digest != fence
    }) {
        bail!("task-protocol conformance synthetic lane roots drifted within the session")
    }
    for values in [
        exchanges
            .iter()
            .map(|item| item.delivery_attempt_digest.as_str())
            .collect::<Vec<_>>(),
        exchanges
            .iter()
            .map(|item| item.exchange_nonce_digest.as_str())
            .collect::<Vec<_>>(),
        exchanges
            .iter()
            .map(|item| item.exchange_root.as_str())
            .collect::<Vec<_>>(),
        exchanges
            .iter()
            .map(|item| item.adapter_observation_id.as_str())
            .collect::<Vec<_>>(),
    ] {
        if values.iter().copied().collect::<BTreeSet<_>>().len() != values.len() {
            bail!("task-protocol conformance per-exchange identity was reused")
        }
    }
    Ok(())
}

fn validate_exchange(
    actual: &TaskProtocolConformanceExchangeObservation,
    expected: &ExternalPoolAdapterTaskProtocolConformanceFixtureExchange,
    index: usize,
) -> Result<()> {
    for value in [
        &actual.command_digest,
        &actual.outbox_operation_digest,
        &actual.route_authorization_digest,
        &actual.synthetic_executor_digest,
        &actual.fence_digest,
        &actual.request_digest,
        &actual.delivery_attempt_digest,
        &actual.exchange_nonce_digest,
        &actual.upstream_request_sha256,
        &actual.upstream_response_sha256,
        &actual.semantic_observation_sha256,
        &actual.exchange_root,
    ] {
        digest(value)?;
    }
    identifier(&actual.adapter_observation_id)?;
    if actual.exchange_ordinal != expected.exchange_ordinal
        || actual.scenario_id != expected.scenario_id
        || actual.operation_kind != expected.operation_kind
        || actual.capability_id != expected.capability_id
        || actual.capability_revision != 1
        || actual.replay_kind != expected.replay_kind
        || actual.response_outcome != "accepted"
        || !expected
            .allowed_state_before
            .contains(&actual.remote_state_before)
        || !expected
            .allowed_state_after
            .contains(&actual.remote_state_after)
        || actual.terminality != expected.terminality
        || actual.upstream_request_bytes == 0
        || actual.upstream_request_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_UPSTREAM_REQUEST_BYTES
        || actual.upstream_response_bytes == 0
        || actual.upstream_response_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_RESPONSE_BYTES
        || actual.semantic_observation_bytes == 0
        || actual.semantic_observation_bytes > TASK_PROTOCOL_CONFORMANCE_MAX_OBSERVATION_BYTES
        || actual.remote_reference_digest.is_some() != expected.reference_required
        || actual.remote_sequence != expected.remote_sequence
        || actual.commit_uncertainty_state_before != expected.commit_uncertainty_state_before
        || actual.commit_uncertainty_state_after != expected.commit_uncertainty_state_after
        || actual.commit_uncertainty_marker_digest.is_some()
            != expected.commit_uncertainty_marker_required
        || actual.event_replay_classification != expected.event_replay_classification
        || actual.event_replay_batch_count != expected.expected_event_replay_batch_count
        || actual.event_replay_root.is_some() != expected.event_replay_root_required
        || actual.oracle_start_count_after != expected.expected_start_count
        || actual.oracle_event_count_after != expected.expected_event_count
    {
        bail!("task-protocol conformance exchange {index} does not match the fixture")
    }
    if let Some(value) = &actual.remote_reference_digest {
        digest(value)?;
    }
    if let Some(value) = &actual.no_commit_tombstone_digest {
        digest(value)?;
    }
    if let Some(value) = &actual.commit_uncertainty_marker_digest {
        digest(value)?;
    }
    if expected.tombstone_required != actual.no_commit_tombstone_digest.is_some() {
        bail!("task-protocol conformance exchange tombstone is not exact")
    }
    validate_event_roots(actual, expected)?;
    let (start_before, event_before) = match actual.exchange_ordinal {
        1 | 2 | 6 | 7 | 8 => (0, 0),
        3 | 4 | 5 => (1, 0),
        _ => bail!("task-protocol conformance exchange ordinal is unsupported"),
    };
    if actual.oracle_start_count_before != start_before
        || actual.oracle_event_count_before != event_before
    {
        bail!("task-protocol conformance oracle counters are not continuous")
    }
    Ok(())
}

fn validate_event_roots(
    actual: &TaskProtocolConformanceExchangeObservation,
    expected: &ExternalPoolAdapterTaskProtocolConformanceFixtureExchange,
) -> Result<()> {
    let is_event_exchange = !expected.event_kinds.is_empty();
    let event_roots_present = actual.event_cursor_before_digest.is_some()
        && actual.event_cursor_after_digest.is_some()
        && actual.event_inventory_digest.is_some();
    let event_roots_absent = actual.event_cursor_before_digest.is_none()
        && actual.event_cursor_after_digest.is_none()
        && actual.event_inventory_digest.is_none();
    if (is_event_exchange && !event_roots_present)
        || (!is_event_exchange && !event_roots_absent)
        || actual.event_count != u64::try_from(expected.event_kinds.len())?
        || (is_event_exchange
            && actual.event_replay_root.as_ref() != actual.event_inventory_digest.as_ref())
    {
        bail!("task-protocol conformance event inventory is not exact")
    }
    for value in [
        &actual.event_cursor_before_digest,
        &actual.event_cursor_after_digest,
        &actual.event_inventory_digest,
        &actual.event_replay_root,
    ]
    .into_iter()
    .flatten()
    {
        digest(value)?;
    }
    Ok(())
}
