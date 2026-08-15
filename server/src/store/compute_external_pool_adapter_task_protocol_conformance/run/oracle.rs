use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

mod uncertainty;

#[cfg(test)]
mod tests;

use crate::compute_federation::external_pool_adapter_task_protocol_conformance::TaskProtocolConformanceExchangeObservation;
use elon_external_pool_adapter_session_core::{
    prepare_external_pool_adapter_task_request, ExternalPoolAdapterTaskProtocolHost,
};

use self::uncertainty::{CommitUncertaintyState, PostReceiptCommitUncertainty};
use super::support::{
    derived_digest, event_root, ExchangeMaterial, CURSOR_DOMAIN, EVENT_INVENTORY_DOMAIN,
    EXACT_RESPONSE_BYTES, OBSERVATION_SCHEMA, REFERENCE_DOMAIN, RESPONSE_SCHEMA, TOMBSTONE_DOMAIN,
};

pub(super) struct StatefulTaskProtocolOracle {
    command_a: ScenarioState,
    command_b: ScenarioState,
}

struct ScenarioState {
    scenario_id: &'static str,
    remote_state: &'static str,
    remote_reference_digest: String,
    remote_sequence: u64,
    start_count: u64,
    event_count: u64,
    cancellation_acknowledged: bool,
    commit_uncertainty: CommitUncertaintyState,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureUpstreamResponse {
    schema: String,
    response_outcome: String,
    remote_state_after: String,
    terminality: String,
    remote_reference_digest: Option<String>,
    remote_sequence: Option<u64>,
    no_commit_tombstone_digest: Option<String>,
    event_cursor_before: Option<String>,
    event_cursor_after: Option<String>,
    event_count: u64,
    event_inventory_digest: Option<String>,
    event_replay_classification: Option<String>,
    event_replay_batch_count: u64,
    event_replay_root: Option<String>,
    oracle_start_count_after: u64,
    oracle_event_count_after: u64,
    events: Vec<FixtureEvent>,
    replayed_events: Vec<FixtureEvent>,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureObservation {
    schema: String,
    adapter_observation_id: String,
    response_outcome: String,
    remote_state_before: String,
    remote_state_after: String,
    terminality: String,
    remote_reference_digest: Option<String>,
    remote_sequence: Option<u64>,
    no_commit_tombstone_digest: Option<String>,
    event_cursor_before: Option<String>,
    event_cursor_after: Option<String>,
    event_count: u64,
    event_inventory_digest: Option<String>,
    commit_uncertainty_state_before: String,
    commit_uncertainty_state_after: String,
    commit_uncertainty_marker_digest: Option<String>,
    event_replay_classification: Option<String>,
    event_replay_batch_count: u64,
    event_replay_root: Option<String>,
    oracle_start_count_before: u64,
    oracle_start_count_after: u64,
    oracle_event_count_before: u64,
    oracle_event_count_after: u64,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureEvent {
    event_sequence: u64,
    event_kind: String,
    previous_event_root: String,
    event_root: String,
}

impl StatefulTaskProtocolOracle {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            command_a: ScenarioState::new("synthetic_command_a")?,
            command_b: ScenarioState::new("synthetic_command_b")?,
        })
    }

    pub(super) fn execute_exchange(
        &mut self,
        protocol: &mut ExternalPoolAdapterTaskProtocolHost<'_>,
        material: ExchangeMaterial,
        timeout: std::time::Duration,
    ) -> Result<TaskProtocolConformanceExchangeObservation> {
        let prepared = prepare_external_pool_adapter_task_request(
            material.operation,
            &material.command_digest,
            &material.outbox_operation_digest,
            &material.route_authorization_digest,
            &material.executor_binding_digest,
            &material.fence_digest,
            &material.body,
        )?;
        let request_digest = prepared.request_digest_hex();
        let exchange = protocol.begin(prepared, &material.delivery_attempt_digest, timeout)?;
        if exchange.request() != material.body.as_slice()
            || exchange.expected_response_bytes() != EXACT_RESPONSE_BYTES
        {
            bail!("task conformance child upstream request is not exact");
        }
        let (response, expected_observation, post_receipt_uncertainty) =
            self.transition(&material, &request_digest)?;
        let response = padded_response(&response)?;
        let (receipt, observed) = exchange.complete(&response, move |observation| {
            let observed: FixtureObservation = serde_json::from_slice(observation)?;
            if observed != expected_observation {
                bail!("task conformance child semantic observation drifted");
            }
            Ok(observed)
        })?;
        if receipt.ordinal() != material.ordinal || receipt.operation() != material.operation {
            bail!("task conformance authenticated receipt identity drifted");
        }
        self.apply_post_receipt_uncertainty(material.scenario_id, post_receipt_uncertainty)?;
        Ok(TaskProtocolConformanceExchangeObservation {
            exchange_ordinal: material.ordinal,
            scenario_id: material.scenario_id.to_owned(),
            operation_kind: material.operation.as_str().to_owned(),
            capability_id: material.capability_id.to_owned(),
            capability_revision: 1,
            replay_kind: material.replay_kind.to_owned(),
            command_digest: receipt.command_digest_hex(),
            outbox_operation_digest: receipt.outbox_operation_digest_hex(),
            route_authorization_digest: receipt.route_authorization_digest_hex(),
            synthetic_executor_digest: receipt.executor_binding_digest_hex(),
            fence_digest: receipt.fence_digest_hex(),
            request_digest: receipt.request_digest_hex(),
            delivery_attempt_digest: receipt.delivery_attempt_digest_hex(),
            exchange_nonce_digest: receipt.exchange_nonce_digest_hex(),
            upstream_request_bytes: u64::from(receipt.upstream_request_bytes()),
            upstream_request_sha256: receipt.upstream_request_sha256_hex(),
            upstream_response_bytes: u64::from(receipt.upstream_response_bytes()),
            upstream_response_sha256: receipt.upstream_response_sha256_hex(),
            semantic_observation_bytes: u64::from(receipt.semantic_observation_bytes()),
            semantic_observation_sha256: receipt.semantic_observation_sha256_hex(),
            exchange_root: receipt.exchange_root_hex(),
            adapter_observation_id: observed.adapter_observation_id,
            response_outcome: observed.response_outcome,
            remote_state_before: observed.remote_state_before,
            remote_state_after: observed.remote_state_after,
            terminality: observed.terminality,
            remote_reference_digest: observed.remote_reference_digest,
            remote_sequence: observed.remote_sequence,
            no_commit_tombstone_digest: observed.no_commit_tombstone_digest,
            event_cursor_before_digest: observed.event_cursor_before,
            event_cursor_after_digest: observed.event_cursor_after,
            event_count: observed.event_count,
            event_inventory_digest: observed.event_inventory_digest,
            commit_uncertainty_state_before: observed.commit_uncertainty_state_before,
            commit_uncertainty_state_after: observed.commit_uncertainty_state_after,
            commit_uncertainty_marker_digest: observed.commit_uncertainty_marker_digest,
            event_replay_classification: observed.event_replay_classification,
            event_replay_batch_count: observed.event_replay_batch_count,
            event_replay_root: observed.event_replay_root,
            oracle_start_count_before: observed.oracle_start_count_before,
            oracle_start_count_after: observed.oracle_start_count_after,
            oracle_event_count_before: observed.oracle_event_count_before,
            oracle_event_count_after: observed.oracle_event_count_after,
        })
    }

    fn transition(
        &mut self,
        material: &ExchangeMaterial,
        request_digest: &str,
    ) -> Result<(
        FixtureUpstreamResponse,
        FixtureObservation,
        PostReceiptCommitUncertainty,
    )> {
        let ordinal = material.ordinal;
        let scenario_id = material.scenario_id;
        let state = match scenario_id {
            "synthetic_command_a" => &mut self.command_a,
            "synthetic_command_b" => &mut self.command_b,
            _ => bail!("task conformance oracle scenario rejected"),
        };
        if state.scenario_id != scenario_id {
            bail!("task conformance oracle scenario identity drifted");
        }
        let start_before = state.start_count;
        let event_before = state.event_count;
        let before = state.remote_state;
        let uncertainty = state.commit_uncertainty.plan(
            material,
            request_digest,
            &state.remote_reference_digest,
            state.remote_sequence,
        )?;
        let (after, terminality, tombstone, events) = match ordinal {
            1 if scenario_id == "synthetic_command_a" && before == "absent" => {
                state.remote_state = "prepared";
                state.remote_sequence = 1;
                ("prepared", "nonterminal", None, Vec::new())
            }
            2 if scenario_id == "synthetic_command_a" && before == "prepared" => {
                state.remote_state = "committed";
                state.remote_sequence = 2;
                state.start_count = 1;
                ("committed", "nonterminal", None, Vec::new())
            }
            3 if scenario_id == "synthetic_command_a"
                && before == "committed"
                && state.remote_sequence == 2
                && state.start_count == 1 =>
            {
                ("committed", "nonterminal", None, Vec::new())
            }
            4 if scenario_id == "synthetic_command_a"
                && before == "committed"
                && state.start_count == 1 =>
            {
                state.remote_state = "running";
                ("running", "nonterminal", None, Vec::new())
            }
            5 if scenario_id == "synthetic_command_a"
                && before == "running"
                && state.start_count == 1
                && state.event_count == 0 =>
            {
                let events = exact_events(scenario_id);
                state.event_count = 2;
                state.remote_state = "terminal_after_run";
                ("terminal_after_run", "final", None, events)
            }
            6 if scenario_id == "synthetic_command_b" && before == "absent" => {
                state.remote_state = "prepared";
                state.remote_sequence = 1;
                ("prepared", "nonterminal", None, Vec::new())
            }
            7 if scenario_id == "synthetic_command_b"
                && before == "prepared"
                && state.start_count == 0
                && state.event_count == 0 =>
            {
                state.cancellation_acknowledged = true;
                ("prepared", "nonterminal", None, Vec::new())
            }
            8 if scenario_id == "synthetic_command_b"
                && before == "prepared"
                && state.cancellation_acknowledged
                && state.start_count == 0
                && state.event_count == 0 =>
            {
                state.remote_state = "terminal_no_start";
                state.remote_sequence = 2;
                (
                    "terminal_no_start",
                    "final",
                    Some(derived_digest(TOMBSTONE_DOMAIN, &[scenario_id.as_bytes()])),
                    Vec::new(),
                )
            }
            _ => bail!("task conformance oracle transition rejected"),
        };
        let (event_cursor_before, event_cursor_after, event_inventory_digest) =
            match events.as_slice() {
                [] => (None, None, None),
                [first, second] => {
                    let before = first.previous_event_root.clone();
                    let after = second.event_root.clone();
                    let inventory = derived_digest(
                        EVENT_INVENTORY_DOMAIN,
                        &[first.event_root.as_bytes(), second.event_root.as_bytes()],
                    );
                    (Some(before), Some(after), Some(inventory))
                }
                _ => bail!("task conformance oracle event inventory shape rejected"),
            };
        let (
            event_replay_classification,
            event_replay_batch_count,
            event_replay_root,
            replayed_events,
        ) = if ordinal == 5 {
            let replay_root = event_inventory_digest
                .clone()
                .ok_or_else(|| anyhow::anyhow!("task conformance event replay lacks inventory"))?;
            (
                Some("exact_duplicate_batch_replay".to_owned()),
                1,
                Some(replay_root),
                events.clone(),
            )
        } else {
            (None, 0, None, Vec::new())
        };
        let response = FixtureUpstreamResponse {
            schema: RESPONSE_SCHEMA.to_owned(),
            response_outcome: "accepted".to_owned(),
            remote_state_after: after.to_owned(),
            terminality: terminality.to_owned(),
            remote_reference_digest: Some(state.remote_reference_digest.clone()),
            remote_sequence: Some(state.remote_sequence),
            no_commit_tombstone_digest: tombstone,
            event_cursor_before,
            event_cursor_after,
            event_count: u64::try_from(events.len())?,
            event_inventory_digest,
            event_replay_classification,
            event_replay_batch_count,
            event_replay_root,
            oracle_start_count_after: state.start_count,
            oracle_event_count_after: state.event_count,
            events,
            replayed_events,
        };
        let observation = FixtureObservation {
            schema: OBSERVATION_SCHEMA.to_owned(),
            adapter_observation_id: format!("synthetic_task_protocol_exchange_{ordinal}"),
            response_outcome: response.response_outcome.clone(),
            remote_state_before: before.to_owned(),
            remote_state_after: response.remote_state_after.clone(),
            terminality: response.terminality.clone(),
            remote_reference_digest: response.remote_reference_digest.clone(),
            remote_sequence: response.remote_sequence,
            no_commit_tombstone_digest: response.no_commit_tombstone_digest.clone(),
            event_cursor_before: response.event_cursor_before.clone(),
            event_cursor_after: response.event_cursor_after.clone(),
            event_count: response.event_count,
            event_inventory_digest: response.event_inventory_digest.clone(),
            commit_uncertainty_state_before: uncertainty.before.to_owned(),
            commit_uncertainty_state_after: uncertainty.after.to_owned(),
            commit_uncertainty_marker_digest: uncertainty.marker_digest,
            event_replay_classification: response.event_replay_classification.clone(),
            event_replay_batch_count: response.event_replay_batch_count,
            event_replay_root: response.event_replay_root.clone(),
            oracle_start_count_before: start_before,
            oracle_start_count_after: response.oracle_start_count_after,
            oracle_event_count_before: event_before,
            oracle_event_count_after: response.oracle_event_count_after,
        };
        Ok((response, observation, uncertainty.post_receipt))
    }

    fn apply_post_receipt_uncertainty(
        &mut self,
        scenario_id: &str,
        transition: PostReceiptCommitUncertainty,
    ) -> Result<()> {
        let state = match scenario_id {
            "synthetic_command_a" => &mut self.command_a,
            "synthetic_command_b" => &mut self.command_b,
            _ => bail!("task conformance post-receipt scenario rejected"),
        };
        state.commit_uncertainty.apply_after_receipt(transition)
    }
}

impl ScenarioState {
    fn new(scenario_id: &'static str) -> Result<Self> {
        Ok(Self {
            scenario_id,
            remote_state: "absent",
            remote_reference_digest: derived_digest(REFERENCE_DOMAIN, &[scenario_id.as_bytes()]),
            remote_sequence: 0,
            start_count: 0,
            event_count: 0,
            cancellation_acknowledged: false,
            commit_uncertainty: CommitUncertaintyState::for_scenario(scenario_id)?,
        })
    }
}

fn exact_events(scenario_id: &str) -> Vec<FixtureEvent> {
    let cursor = derived_digest(CURSOR_DOMAIN, &[scenario_id.as_bytes(), b"before"]);
    let started = event_root(scenario_id, 1, "started", &cursor);
    let terminal = event_root(scenario_id, 2, "terminal", &started);
    vec![
        FixtureEvent {
            event_sequence: 1,
            event_kind: "started".to_owned(),
            previous_event_root: cursor,
            event_root: started.clone(),
        },
        FixtureEvent {
            event_sequence: 2,
            event_kind: "terminal".to_owned(),
            previous_event_root: started,
            event_root: terminal,
        },
    ]
}

fn padded_response(value: &FixtureUpstreamResponse) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    if bytes.len() >= EXACT_RESPONSE_BYTES {
        bail!("task conformance fixture response exceeds exact frame size");
    }
    bytes.resize(EXACT_RESPONSE_BYTES, b' ');
    Ok(bytes)
}
