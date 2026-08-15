use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use super::{
    FixtureObservation, FixtureRequest, FixtureUpstreamResponse, OBSERVATION_SCHEMA,
    RESPONSE_SCHEMA,
};

const REFERENCE_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.reference.v1\0";
const TOMBSTONE_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.no_commit_tombstone.v1\0";
const CURSOR_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.event_cursor.v1\0";
const EVENT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.event.v1\0";
const EVENT_INVENTORY_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.event_inventory.v1\0";
const COMMIT_UNCERTAINTY_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.commit_uncertainty.v1\0";

pub(super) struct FixtureTaskProtocolOracle {
    commit_uncertainty: CommitUncertaintyState,
}

enum CommitUncertaintyState {
    Clear,
    Pending(String),
    Resolved(String),
}

struct CommitUncertaintyPlan {
    before: &'static str,
    after: &'static str,
    marker_digest: Option<String>,
    next: Option<CommitUncertaintyState>,
}

impl FixtureTaskProtocolOracle {
    pub(super) fn new() -> Self {
        Self {
            commit_uncertainty: CommitUncertaintyState::Clear,
        }
    }

    pub(super) fn validate_response(
        &mut self,
        ordinal: u64,
        request: &FixtureRequest,
        request_digest: &str,
        value: &FixtureUpstreamResponse,
    ) -> Result<FixtureObservation> {
        let expected = ExpectedResponse::for_ordinal(ordinal, &request.scenario_id)?;
        if value.schema != RESPONSE_SCHEMA
            || value.response_outcome != "accepted"
            || value.remote_state_after != expected.after
            || value.terminality != expected.terminality
            || value.remote_reference_digest.as_deref() != Some(expected.reference.as_str())
            || value.remote_sequence != Some(expected.remote_sequence)
            || value.no_commit_tombstone_digest != expected.tombstone
            || value.event_count != expected.event_count
            || value.oracle_start_count_after != expected.start_after
            || value.oracle_event_count_after != expected.event_after
        {
            bail!("task conformance fixture response semantics rejected");
        }
        if ordinal == 5 {
            validate_event_chain(&request.scenario_id, value)?;
        } else if value.event_cursor_before.is_some()
            || value.event_cursor_after.is_some()
            || value.event_inventory_digest.is_some()
            || value.event_replay_classification.is_some()
            || value.event_replay_batch_count != 0
            || value.event_replay_root.is_some()
            || !value.events.is_empty()
            || !value.replayed_events.is_empty()
        {
            bail!("task conformance non-event response carried event authority");
        }
        let uncertainty = self.plan_uncertainty(ordinal, request, request_digest, &expected)?;
        let observation = FixtureObservation {
            schema: OBSERVATION_SCHEMA.to_owned(),
            adapter_observation_id: format!("synthetic_task_protocol_exchange_{ordinal}"),
            response_outcome: value.response_outcome.clone(),
            remote_state_before: expected.before.to_owned(),
            remote_state_after: value.remote_state_after.clone(),
            terminality: value.terminality.clone(),
            remote_reference_digest: value.remote_reference_digest.clone(),
            remote_sequence: value.remote_sequence,
            no_commit_tombstone_digest: value.no_commit_tombstone_digest.clone(),
            event_cursor_before: value.event_cursor_before.clone(),
            event_cursor_after: value.event_cursor_after.clone(),
            event_count: value.event_count,
            event_inventory_digest: value.event_inventory_digest.clone(),
            commit_uncertainty_state_before: uncertainty.before.to_owned(),
            commit_uncertainty_state_after: uncertainty.after.to_owned(),
            commit_uncertainty_marker_digest: uncertainty.marker_digest,
            event_replay_classification: value.event_replay_classification.clone(),
            event_replay_batch_count: value.event_replay_batch_count,
            event_replay_root: value.event_replay_root.clone(),
            oracle_start_count_before: expected.start_before,
            oracle_start_count_after: value.oracle_start_count_after,
            oracle_event_count_before: expected.event_before,
            oracle_event_count_after: value.oracle_event_count_after,
        };
        if let Some(next) = uncertainty.next {
            self.commit_uncertainty = next;
        }
        Ok(observation)
    }

    fn plan_uncertainty(
        &self,
        ordinal: u64,
        request: &FixtureRequest,
        request_digest: &str,
        expected: &ExpectedResponse,
    ) -> Result<CommitUncertaintyPlan> {
        let value = match (&self.commit_uncertainty, ordinal) {
            (CommitUncertaintyState::Clear, 1 | 2) => CommitUncertaintyPlan {
                before: "clear",
                after: "clear",
                marker_digest: None,
                next: None,
            },
            (CommitUncertaintyState::Clear, 3) => {
                let marker = commit_uncertainty_marker(
                    request,
                    request_digest,
                    &expected.reference,
                    expected.remote_sequence,
                );
                CommitUncertaintyPlan {
                    before: "clear",
                    after: "unknown_after_remote_acceptance",
                    marker_digest: Some(marker.clone()),
                    next: Some(CommitUncertaintyState::Pending(marker)),
                }
            }
            (CommitUncertaintyState::Pending(marker), 4) => CommitUncertaintyPlan {
                before: "unknown_after_remote_acceptance",
                after: "resolved_by_reconcile",
                marker_digest: Some(marker.clone()),
                next: Some(CommitUncertaintyState::Resolved(marker.clone())),
            },
            (CommitUncertaintyState::Resolved(marker), 5) if !marker.is_empty() => {
                CommitUncertaintyPlan {
                    before: "resolved_by_reconcile",
                    after: "resolved_by_reconcile",
                    marker_digest: None,
                    next: None,
                }
            }
            (CommitUncertaintyState::Resolved(marker), 6..=8) if !marker.is_empty() => {
                CommitUncertaintyPlan {
                    before: "not_applicable",
                    after: "not_applicable",
                    marker_digest: None,
                    next: None,
                }
            }
            _ => bail!("task conformance fixture uncertainty state gate rejected"),
        };
        Ok(value)
    }
}

fn validate_event_chain(scenario_id: &str, value: &FixtureUpstreamResponse) -> Result<()> {
    let [first, second] = value.events.as_slice() else {
        bail!("task conformance event inventory shape rejected");
    };
    let cursor_before = derived_digest(CURSOR_DOMAIN, &[scenario_id.as_bytes(), b"before"]);
    let first_root = event_root(scenario_id, 1, "started", &cursor_before);
    let second_root = event_root(scenario_id, 2, "terminal", &first_root);
    let inventory = derived_digest(
        EVENT_INVENTORY_DOMAIN,
        &[first_root.as_bytes(), second_root.as_bytes()],
    );
    if value.event_cursor_before.as_deref() != Some(cursor_before.as_str())
        || value.event_cursor_after.as_deref() != Some(second_root.as_str())
        || value.event_inventory_digest.as_deref() != Some(inventory.as_str())
        || value.event_replay_classification.as_deref() != Some("exact_duplicate_batch_replay")
        || value.event_replay_batch_count != 1
        || value.event_replay_root.as_deref() != Some(inventory.as_str())
        || value.replayed_events.as_slice() != value.events.as_slice()
        || first.event_sequence != 1
        || first.event_kind != "started"
        || first.previous_event_root != cursor_before
        || first.event_root != first_root
        || second.event_sequence != 2
        || second.event_kind != "terminal"
        || second.previous_event_root != first_root
        || second.event_root != second_root
    {
        bail!("task conformance event cursor/hash chain rejected");
    }
    Ok(())
}

fn commit_uncertainty_marker(
    request: &FixtureRequest,
    request_digest: &str,
    remote_reference_digest: &str,
    remote_sequence: u64,
) -> String {
    derived_digest(
        COMMIT_UNCERTAINTY_DOMAIN,
        &[
            request.command_digest.as_bytes(),
            request.outbox_operation_digest.as_bytes(),
            request.route_authorization_digest.as_bytes(),
            request.synthetic_executor_digest.as_bytes(),
            request.fence_digest.as_bytes(),
            request_digest.as_bytes(),
            remote_reference_digest.as_bytes(),
            &remote_sequence.to_be_bytes(),
        ],
    )
}

struct ExpectedResponse {
    before: &'static str,
    after: &'static str,
    terminality: &'static str,
    reference: String,
    remote_sequence: u64,
    tombstone: Option<String>,
    event_count: u64,
    start_before: u64,
    start_after: u64,
    event_before: u64,
    event_after: u64,
}

impl ExpectedResponse {
    fn for_ordinal(ordinal: u64, scenario_id: &str) -> Result<Self> {
        let reference = derived_digest(REFERENCE_DOMAIN, &[scenario_id.as_bytes()]);
        let (before, after, terminality, remote_sequence, tombstone, event_count, starts, events) =
            match ordinal {
                1 => (
                    "absent",
                    "prepared",
                    "nonterminal",
                    1,
                    None,
                    0,
                    (0, 0),
                    (0, 0),
                ),
                2 => (
                    "prepared",
                    "committed",
                    "nonterminal",
                    2,
                    None,
                    0,
                    (0, 1),
                    (0, 0),
                ),
                3 => (
                    "committed",
                    "committed",
                    "nonterminal",
                    2,
                    None,
                    0,
                    (1, 1),
                    (0, 0),
                ),
                4 => (
                    "committed",
                    "running",
                    "nonterminal",
                    2,
                    None,
                    0,
                    (1, 1),
                    (0, 0),
                ),
                5 => (
                    "running",
                    "terminal_after_run",
                    "final",
                    2,
                    None,
                    2,
                    (1, 1),
                    (0, 2),
                ),
                6 => (
                    "absent",
                    "prepared",
                    "nonterminal",
                    1,
                    None,
                    0,
                    (0, 0),
                    (0, 0),
                ),
                7 => (
                    "prepared",
                    "prepared",
                    "nonterminal",
                    1,
                    None,
                    0,
                    (0, 0),
                    (0, 0),
                ),
                8 => (
                    "prepared",
                    "terminal_no_start",
                    "final",
                    2,
                    Some(derived_digest(TOMBSTONE_DOMAIN, &[scenario_id.as_bytes()])),
                    0,
                    (0, 0),
                    (0, 0),
                ),
                _ => bail!("task conformance fixture response ordinal rejected"),
            };
        Ok(Self {
            before,
            after,
            terminality,
            reference,
            remote_sequence,
            tombstone,
            event_count,
            start_before: starts.0,
            start_after: starts.1,
            event_before: events.0,
            event_after: events.1,
        })
    }
}

fn event_root(scenario_id: &str, sequence: u64, kind: &str, previous: &str) -> String {
    derived_digest(
        EVENT_DOMAIN,
        &[
            scenario_id.as_bytes(),
            &sequence.to_be_bytes(),
            kind.as_bytes(),
            previous.as_bytes(),
        ],
    )
}

fn derived_digest(domain: &[u8], fields: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    for field in fields {
        digest.update((field.len() as u64).to_be_bytes());
        digest.update(field);
    }
    hex::encode(digest.finalize())
}
