use std::{fs::File, os::unix::fs::FileExt};

use anyhow::{bail, Result};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::compute_federation::{
    external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    external_pool_adapter_runtime_compatibility_verification::{
        RUNTIME_COMPATIBILITY_CONFIG_FIXTURE_PATH, RUNTIME_COMPATIBILITY_CREDENTIAL_FIXTURE_PATH,
        RUNTIME_COMPATIBILITY_NO_WORK_REQUEST_FIXTURE_PATH,
        RUNTIME_COMPATIBILITY_NO_WORK_RESPONSE_FIXTURE_PATH,
    },
    external_pool_adapter_task_protocol_conformance::{
        task_protocol_conformance_capability_assertion_inventory_digest,
        task_protocol_conformance_capability_fixture_digest,
        task_protocol_conformance_exchange_inventory_digest,
        TaskProtocolConformanceCapabilityObservation, TaskProtocolConformanceExchangeObservation,
        TASK_PROTOCOL_CONFORMANCE_CAPABILITY_IDS,
        TASK_PROTOCOL_CONFORMANCE_COMMIT_UNCERTAINTY_DOMAIN,
    },
};

use super::TaskProtocolConformanceFixtureResourceIdentity;
use elon_external_pool_adapter_session_core::ExternalPoolAdapterTaskOperationKind;

pub(super) const EXACT_RESPONSE_BYTES: usize = 2_048;
pub(super) const REQUEST_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_request.v1";
pub(super) const RESPONSE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_response.v1";
pub(super) const OBSERVATION_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_conformance_fixture_observation.v1";
const RUN_NONCE_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.run_nonce.v1\0";
const COMMAND_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.command.v1\0";
const OUTBOX_OPERATION_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.outbox_operation.v1\0";
const ROUTE_AUTHORIZATION_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.route_authorization.v1\0";
const FENCE_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.fence.v1\0";
const DELIVERY_ATTEMPT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.delivery_attempt.v1\0";
pub(super) const REFERENCE_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.reference.v1\0";
pub(super) const TOMBSTONE_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.no_commit_tombstone.v1\0";
pub(super) const CURSOR_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.event_cursor.v1\0";
pub(super) const EVENT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.event.v1\0";
pub(super) const EVENT_INVENTORY_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.fixture.event_inventory.v1\0";

pub(super) struct PublicFixtureBytes {
    pub(super) config: Zeroizing<Vec<u8>>,
    pub(super) credential: Zeroizing<Vec<u8>>,
}

pub(super) struct ExchangeMaterial {
    pub(super) ordinal: u64,
    pub(super) scenario_id: &'static str,
    pub(super) operation: ExternalPoolAdapterTaskOperationKind,
    pub(super) capability_id: &'static str,
    pub(super) replay_kind: &'static str,
    pub(super) command_digest: String,
    pub(super) outbox_operation_digest: String,
    pub(super) route_authorization_digest: String,
    pub(super) executor_binding_digest: String,
    pub(super) fence_digest: String,
    pub(super) delivery_attempt_digest: String,
    pub(super) body: Zeroizing<Vec<u8>>,
}

#[derive(Serialize)]
struct FixtureRequest<'a> {
    schema: &'static str,
    scenario_id: &'static str,
    operation_kind: &'static str,
    command_digest: &'a str,
    outbox_operation_digest: &'a str,
    route_authorization_digest: &'a str,
    synthetic_executor_digest: &'a str,
    fence_digest: &'a str,
}

pub(super) fn run_nonce_digest() -> Result<String> {
    let mut nonce = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow::anyhow!("generate task conformance run nonce"))?;
    if nonce.iter().all(|byte| *byte == 0) {
        bail!("task conformance run nonce rejected");
    }
    let output = derived_digest(RUN_NONCE_DOMAIN, &[&nonce]);
    nonce.zeroize();
    Ok(output)
}

pub(super) fn exchange_material(
    ordinal: u64,
    run_nonce_digest: &str,
    fixture_lane_digest: &str,
    fixture_executor_digest: &str,
) -> Result<ExchangeMaterial> {
    let (scenario_id, operation, capability_id, replay_kind, operation_identity) = match ordinal {
        1 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::Prepare,
            "prepare",
            "fresh",
            "a_prepare",
        ),
        2 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::IdempotentCommit,
            "idempotent_commit",
            "fresh",
            "a_commit",
        ),
        3 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::IdempotentCommit,
            "idempotent_commit",
            "same_idempotency_exact_replay",
            "a_commit",
        ),
        4 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::Reconcile,
            "reconcile",
            "fresh",
            "a_reconcile",
        ),
        5 => (
            "synthetic_command_a",
            ExternalPoolAdapterTaskOperationKind::AuthenticatedEvents,
            "authenticated_events",
            "fresh",
            "a_events",
        ),
        6 => (
            "synthetic_command_b",
            ExternalPoolAdapterTaskOperationKind::Prepare,
            "prepare",
            "fresh",
            "b_prepare",
        ),
        7 => (
            "synthetic_command_b",
            ExternalPoolAdapterTaskOperationKind::CancelNoStart,
            "cancel_no_start",
            "fresh",
            "b_cancel",
        ),
        8 => (
            "synthetic_command_b",
            ExternalPoolAdapterTaskOperationKind::Reconcile,
            "reconcile",
            "fresh",
            "b_reconcile",
        ),
        _ => bail!("task conformance exchange ordinal rejected"),
    };
    let command_digest = derived_digest(COMMAND_DOMAIN, &[scenario_id.as_bytes()]);
    let outbox_operation_digest = derived_digest(
        OUTBOX_OPERATION_DOMAIN,
        &[scenario_id.as_bytes(), operation_identity.as_bytes()],
    );
    let route_authorization_digest = derived_digest(
        ROUTE_AUTHORIZATION_DOMAIN,
        &[fixture_lane_digest.as_bytes()],
    );
    let fence_digest = derived_digest(FENCE_DOMAIN, &[fixture_lane_digest.as_bytes()]);
    let delivery_attempt_digest = derived_digest(
        DELIVERY_ATTEMPT_DOMAIN,
        &[run_nonce_digest.as_bytes(), &ordinal.to_be_bytes()],
    );
    let body = serde_json::to_vec(&FixtureRequest {
        schema: REQUEST_SCHEMA,
        scenario_id,
        operation_kind: operation.as_str(),
        command_digest: &command_digest,
        outbox_operation_digest: &outbox_operation_digest,
        route_authorization_digest: &route_authorization_digest,
        synthetic_executor_digest: fixture_executor_digest,
        fence_digest: &fence_digest,
    })?;
    Ok(ExchangeMaterial {
        ordinal,
        scenario_id,
        operation,
        capability_id,
        replay_kind,
        command_digest,
        outbox_operation_digest,
        route_authorization_digest,
        executor_binding_digest: fixture_executor_digest.to_owned(),
        fence_digest,
        delivery_attempt_digest,
        body: Zeroizing::new(body),
    })
}

pub(super) fn load_public_fixtures(
    prepared: &PreparedExternalPoolAdapterInstallation,
    resources: &[TaskProtocolConformanceFixtureResourceIdentity],
) -> Result<PublicFixtureBytes> {
    let expected_inventory = [
        ("config", RUNTIME_COMPATIBILITY_CONFIG_FIXTURE_PATH),
        ("credential", RUNTIME_COMPATIBILITY_CREDENTIAL_FIXTURE_PATH),
        (
            "no_work_request",
            RUNTIME_COMPATIBILITY_NO_WORK_REQUEST_FIXTURE_PATH,
        ),
        (
            "no_work_response",
            RUNTIME_COMPATIBILITY_NO_WORK_RESPONSE_FIXTURE_PATH,
        ),
    ];
    if resources.len() != expected_inventory.len() {
        bail!("task conformance V268 fixture inventory is not exact");
    }
    let mut config = None;
    let mut credential = None;
    for (resource, (expected_purpose, expected_path)) in resources.iter().zip(expected_inventory) {
        if resource.purpose.trim() != resource.purpose
            || resource.purpose.is_empty()
            || resource.path.trim() != resource.path
            || resource.path.is_empty()
            || resource.purpose != expected_purpose
            || resource.path != expected_path
            || resource.size_bytes == 0
            || !is_digest(&resource.sha256)
        {
            bail!("task conformance V268 fixture identity is invalid");
        }
        let bytes = load_fixture(prepared, resource)?;
        match resource.purpose.as_str() {
            "config" if config.is_none() => config = Some(bytes),
            "credential" if credential.is_none() => credential = Some(bytes),
            "config" | "credential" => {
                bail!("task conformance V268 fixture purpose is duplicated")
            }
            _ => {}
        }
    }
    Ok(PublicFixtureBytes {
        config: config.ok_or_else(|| anyhow::anyhow!("task conformance config fixture missing"))?,
        credential: credential
            .ok_or_else(|| anyhow::anyhow!("task conformance credential fixture missing"))?,
    })
}

pub(super) fn capability_observations(
    exchanges: &[TaskProtocolConformanceExchangeObservation],
    profile_digest: &str,
    fixture_catalog_digest: &str,
) -> Result<Vec<TaskProtocolConformanceCapabilityObservation>> {
    let ordinal_sets = [
        vec![1, 2, 3, 4, 5, 6, 7, 8],
        vec![5],
        vec![7, 8],
        vec![2, 3],
        vec![1, 6],
        vec![4, 8],
    ];
    TASK_PROTOCOL_CONFORMANCE_CAPABILITY_IDS
        .into_iter()
        .zip(ordinal_sets)
        .map(|(capability_id, exchange_ordinals)| {
            let selected: Vec<_> = exchange_ordinals
                .iter()
                .map(|ordinal| {
                    let index = ordinal
                        .checked_sub(1)
                        .and_then(|value| usize::try_from(value).ok())
                        .ok_or_else(|| {
                            anyhow::anyhow!("task conformance capability ordinal overflow")
                        })?;
                    exchanges.get(index).cloned().ok_or_else(|| {
                        anyhow::anyhow!("task conformance capability exchange missing")
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let exchange_inventory_digest =
                task_protocol_conformance_exchange_inventory_digest(&selected)?;
            let fixture_digest = task_protocol_conformance_capability_fixture_digest(
                profile_digest,
                fixture_catalog_digest,
                capability_id,
                1,
                &exchange_ordinals,
            )?;
            let test_case_id =
                format!("external_pool_adapter_task_protocol_conformance_{capability_id}_v1");
            let status = "passed_server_run".to_owned();
            let assertion_inventory_digest =
                task_protocol_conformance_capability_assertion_inventory_digest(
                    capability_id,
                    1,
                    &status,
                    &test_case_id,
                    &fixture_digest,
                    &exchange_ordinals,
                    &exchange_inventory_digest,
                )?;
            Ok(TaskProtocolConformanceCapabilityObservation {
                capability_id: capability_id.to_owned(),
                capability_revision: 1,
                status,
                test_case_id,
                fixture_digest,
                exchange_ordinals,
                exchange_inventory_digest,
                assertion_inventory_digest,
            })
        })
        .collect()
}

pub(super) fn derived_digest(domain: &[u8], fields: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    for field in fields {
        digest.update((field.len() as u64).to_be_bytes());
        digest.update(field);
    }
    hex::encode(digest.finalize())
}

pub(super) fn commit_uncertainty_marker_digest(
    material: &ExchangeMaterial,
    request_digest: &str,
    remote_reference_digest: &str,
    remote_sequence: u64,
) -> String {
    derived_digest(
        TASK_PROTOCOL_CONFORMANCE_COMMIT_UNCERTAINTY_DOMAIN,
        &[
            material.command_digest.as_bytes(),
            material.outbox_operation_digest.as_bytes(),
            material.route_authorization_digest.as_bytes(),
            material.executor_binding_digest.as_bytes(),
            material.fence_digest.as_bytes(),
            request_digest.as_bytes(),
            remote_reference_digest.as_bytes(),
            &remote_sequence.to_be_bytes(),
        ],
    )
}

pub(super) fn event_root(scenario_id: &str, sequence: u64, kind: &str, previous: &str) -> String {
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

fn load_fixture(
    prepared: &PreparedExternalPoolAdapterInstallation,
    expected: &TaskProtocolConformanceFixtureResourceIdentity,
) -> Result<Zeroizing<Vec<u8>>> {
    let (file, sha256, size_bytes) = prepared.retained_resource(&expected.path)?;
    if sha256 != expected.sha256 || size_bytes != expected.size_bytes {
        bail!("task conformance retained V268 fixture identity drifted");
    }
    let mut bytes = Zeroizing::new(vec![0_u8; usize::try_from(size_bytes)?]);
    read_exact_at(file, &mut bytes, size_bytes)?;
    if hex::encode(Sha256::digest(&bytes[..])) != expected.sha256 {
        bail!("task conformance retained V268 fixture bytes drifted");
    }
    Ok(bytes)
}

fn read_exact_at(file: &File, mut output: &mut [u8], expected_size: u64) -> Result<()> {
    if file.metadata()?.len() != expected_size {
        bail!("task conformance retained fixture length drifted");
    }
    let mut offset = 0_u64;
    while !output.is_empty() {
        match file.read_at(output, offset)? {
            0 => bail!("task conformance retained fixture ended early"),
            read => {
                output = &mut output[read..];
                offset += u64::try_from(read)?;
            }
        }
    }
    Ok(())
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        && value.bytes().any(|byte| byte != b'0')
}
