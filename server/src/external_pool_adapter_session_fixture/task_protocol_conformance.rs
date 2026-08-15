use std::time::Duration;

#[path = "task_protocol_conformance/oracle.rs"]
mod oracle;

use anyhow::{anyhow, bail, Result};
use elon_external_pool_adapter_session_core::{
    AuthenticatedExternalPoolAdapterSession, DeliveredExternalPoolAdapterEphemeralBundle,
    ExternalPoolAdapterSessionRoots, ExternalPoolAdapterTaskOperationKind,
    ExternalPoolAdapterTaskProtocolChild,
};
use serde::{Deserialize, Serialize};

const EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000);
const EXACT_RESPONSE_BYTES: usize = 2_048;
const REQUEST_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_request.v1";
const RESPONSE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_response.v1";
const OBSERVATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_observation.v1";

const ROOT_ARGUMENT_PREFIXES: [&str; 14] = [
    "--elon-task-protocol-conformance-session-policy=",
    "--elon-task-protocol-conformance-profile=",
    "--elon-task-protocol-conformance-run-nonce=",
    "--elon-task-protocol-conformance-fixture-catalog=",
    "--elon-task-protocol-conformance-registry-release=",
    "--elon-task-protocol-conformance-installation-content=",
    "--elon-task-protocol-conformance-capability-set=",
    "--elon-task-protocol-conformance-sandbox-reattestation-receipt=",
    "--elon-task-protocol-conformance-runtime-compatibility-receipt=",
    "--elon-task-protocol-conformance-source-capsule=",
    "--elon-task-protocol-conformance-launch-image=",
    "--elon-task-protocol-conformance-public-delivery=",
    "--elon-task-protocol-conformance-synthetic-fixture-lane=",
    "--elon-task-protocol-conformance-synthetic-fixture-executor=",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureRequest {
    schema: String,
    scenario_id: String,
    operation_kind: String,
    command_digest: String,
    outbox_operation_digest: String,
    route_authorization_digest: String,
    synthetic_executor_digest: String,
    fence_digest: String,
}

#[derive(Deserialize, Serialize)]
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

#[derive(Serialize)]
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

pub(super) fn parse_roots(
    arguments: &[String],
) -> Result<(ExternalPoolAdapterSessionRoots, String)> {
    if arguments.len() != ROOT_ARGUMENT_PREFIXES.len() {
        bail!("fixed task conformance root argument count rejected");
    }
    let values: Vec<&str> = arguments
        .iter()
        .zip(ROOT_ARGUMENT_PREFIXES)
        .map(|(argument, prefix)| {
            argument
                .strip_prefix(prefix)
                .ok_or_else(|| anyhow!("fixed task conformance root argument prefix rejected"))
        })
        .collect::<Result<_>>()?;
    let values: [&str; 14] = values
        .try_into()
        .map_err(|_| anyhow!("fixed task conformance root value count rejected"))?;
    let [supervisor_session_policy_digest, task_protocol_profile_digest, run_nonce_digest, fixture_catalog_digest, registry_release_digest, installation_content_digest, capability_set_digest, sandbox_reattestation_receipt_digest, runtime_compatibility_verification_receipt_digest, source_capsule_sha256, launch_image_sha256, public_fixture_delivery_root, synthetic_fixture_lane_digest, synthetic_fixture_executor_digest] =
        values;
    Ok((
        ExternalPoolAdapterSessionRoots::new_task_protocol_conformance(
            supervisor_session_policy_digest,
            task_protocol_profile_digest,
            run_nonce_digest,
            fixture_catalog_digest,
            registry_release_digest,
            installation_content_digest,
            capability_set_digest,
            sandbox_reattestation_receipt_digest,
            runtime_compatibility_verification_receipt_digest,
            source_capsule_sha256,
            launch_image_sha256,
            public_fixture_delivery_root,
            synthetic_fixture_lane_digest,
            synthetic_fixture_executor_digest,
        )?,
        public_fixture_delivery_root.to_owned(),
    ))
}

pub(super) fn execute(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    delivered: DeliveredExternalPoolAdapterEphemeralBundle,
    expected_config: &[u8],
    expected_credential: &[u8],
) -> Result<()> {
    if delivered.config() != expected_config || delivered.credential() != expected_credential {
        bail!("task conformance exact public fixture material rejected");
    }
    let mut protocol = ExternalPoolAdapterTaskProtocolChild::new(session);
    let mut oracle = oracle::FixtureTaskProtocolOracle::new();
    for expected_ordinal in 1..=8 {
        let exchange = protocol.next(EXCHANGE_TIMEOUT)?;
        if exchange.ordinal() != expected_ordinal {
            bail!("task conformance exchange order drifted");
        }
        let request: FixtureRequest = serde_json::from_slice(exchange.request_body())?;
        validate_request(&exchange, &request)?;
        let request_digest = exchange.request_digest_hex();
        let request_bytes = exchange.request_body().to_vec();
        exchange.complete(&request_bytes, EXACT_RESPONSE_BYTES, |response| {
            let response = parse_padded_response(response)?;
            let observation =
                oracle.validate_response(expected_ordinal, &request, &request_digest, &response)?;
            serde_json::to_vec(&observation).map_err(Into::into)
        })?;
    }
    drop(protocol);
    delivered.wait_for_shutdown(session)
}

fn validate_request(
    exchange: &elon_external_pool_adapter_session_core::ExternalPoolAdapterTaskProtocolChildExchange<'_>,
    request: &FixtureRequest,
) -> Result<()> {
    let (scenario_id, operation) = expected_request(exchange.ordinal())?;
    if request.schema != REQUEST_SCHEMA
        || request.scenario_id != scenario_id
        || request.operation_kind != operation.as_str()
        || exchange.operation() != operation
        || request.command_digest != exchange.command_digest_hex()
        || request.outbox_operation_digest != exchange.outbox_operation_digest_hex()
        || request.route_authorization_digest != exchange.route_authorization_digest_hex()
        || request.synthetic_executor_digest != exchange.executor_binding_digest_hex()
        || request.fence_digest != exchange.fence_digest_hex()
    {
        bail!("task conformance fixture request semantics rejected");
    }
    Ok(())
}

fn parse_padded_response(bytes: &[u8]) -> Result<FixtureUpstreamResponse> {
    if bytes.len() != EXACT_RESPONSE_BYTES {
        bail!("task conformance response length rejected");
    }
    let text = std::str::from_utf8(bytes)?;
    let json = text.trim_end_matches(' ');
    if json.is_empty() || json.len() == text.len() {
        bail!("task conformance padded response rejected");
    }
    serde_json::from_str(json).map_err(Into::into)
}

fn expected_request(ordinal: u64) -> Result<(&'static str, ExternalPoolAdapterTaskOperationKind)> {
    let value = match ordinal {
        1 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::Prepare,
        ),
        2 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::IdempotentCommit,
        ),
        3 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::IdempotentCommit,
        ),
        4 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::Reconcile,
        ),
        5 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::AuthenticatedEvents,
        ),
        6 => (
            "synthetic_command_b",
            ExternalPoolAdapterTaskOperationKind::Prepare,
        ),
        7 => (
            "synthetic_command_b",
            ExternalPoolAdapterTaskOperationKind::CancelNoStart,
        ),
        8 => (
            "synthetic_command_b",
            ExternalPoolAdapterTaskOperationKind::Reconcile,
        ),
        _ => bail!("task conformance fixture ordinal rejected"),
    };
    Ok(value)
}
