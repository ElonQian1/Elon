use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::*;

const PROFILE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-PROFILE-V1";
const FIXTURE_CATALOG_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-FIXTURE-CATALOG-V1";
const RUN_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-RUN-MATERIAL-V1";
const RUN_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-RUN-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-REVOCATION-MATERIAL-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-REVOCATION-RECEIPT-V1";
const RECEIPT_INTEGRITY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-RECEIPT-INTEGRITY-V1";
const CAPABILITY_FIXTURE_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-CAPABILITY-FIXTURE-V1";
const EXCHANGE_INVENTORY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-EXCHANGE-INVENTORY-V1";
const DELIVERY_INVENTORY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-DELIVERY-INVENTORY-V1";
const TASK_OBSERVATION_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-TASK-OBSERVATION-V1";
const CAPABILITY_ASSERTION_INVENTORY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-CONFORMANCE-CAPABILITY-ASSERTION-INVENTORY-V1";
const FIXTURE_LANE_SUBJECT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.synthetic_fixture_lane.v1\0";
const FIXTURE_EXECUTOR_SUBJECT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.task_protocol_conformance.synthetic_fixture_executor.v1\0";

pub(crate) fn task_protocol_conformance_profile_digest(
    value: &ExternalPoolAdapterTaskProtocolConformanceProfile,
) -> Result<String> {
    domain_digest(PROFILE_DOMAIN, value)
}

pub(crate) fn task_protocol_conformance_fixture_catalog_digest(
    value: &ExternalPoolAdapterTaskProtocolConformanceFixtureCatalog,
) -> Result<String> {
    domain_digest(FIXTURE_CATALOG_DOMAIN, value)
}

pub(crate) fn task_protocol_conformance_run_material_digest(
    value: &ExternalPoolAdapterTaskProtocolConformanceRunMaterial,
) -> Result<String> {
    domain_digest(RUN_MATERIAL_DOMAIN, value)
}

pub(crate) fn task_protocol_conformance_revocation_material_digest(
    value: &ExternalPoolAdapterTaskProtocolConformanceRevocationMaterial,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_task_protocol_conformance_run_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRunReceipt,
) -> Result<(String, String)> {
    receipt_digest(
        receipt,
        "run_receipt_digest",
        RUN_RECEIPT_DOMAIN,
        "task-protocol conformance run receipt",
    )
}

pub(crate) fn canonical_task_protocol_conformance_revocation_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt,
) -> Result<(String, String)> {
    receipt_digest(
        receipt,
        "revocation_receipt_digest",
        REVOCATION_RECEIPT_DOMAIN,
        "task-protocol conformance revocation receipt",
    )
}

/// Derive the exact ELTP v1 transcript root from the frozen 14 raw SHA-256 roots.
pub(crate) fn task_protocol_conformance_session_roots_digest(roots: &[String]) -> Result<String> {
    if roots.len() != TASK_PROTOCOL_CONFORMANCE_SESSION_ROOT_COUNT {
        bail!("task-protocol conformance session root count is not exact")
    }
    let mut digest = Sha256::new();
    digest.update(TASK_PROTOCOL_CONFORMANCE_SESSION_ROOTS_DOMAIN);
    for root in roots {
        digest.update(decode_digest(root)?);
    }
    Ok(hex::encode(digest.finalize()))
}

/// Durable integrity framing for Store-private process custody. The canonical run receipt does
/// not contain any of these private custody values.
pub(crate) fn task_protocol_conformance_receipt_integrity_digest(
    run_receipt_digest: &str,
    runtime_custody_epoch_digest: &str,
    process_hmac_seal: &str,
) -> Result<String> {
    let schema = TASK_PROTOCOL_CONFORMANCE_RUN_RECEIPT_SCHEMA.as_bytes();
    let schema_len = u16::try_from(schema.len())?;
    let mut digest = Sha256::new();
    digest.update(RECEIPT_INTEGRITY_DOMAIN);
    digest.update([0]);
    digest.update(1_u16.to_be_bytes());
    digest.update(schema_len.to_be_bytes());
    digest.update(schema);
    digest.update(decode_digest(run_receipt_digest)?);
    digest.update(decode_digest(runtime_custody_epoch_digest)?);
    digest.update(decode_digest(process_hmac_seal)?);
    Ok(hex::encode(digest.finalize()))
}

/// Stable marker for the authenticated accepted commit replay whose caller-visible outcome is
/// deliberately treated as unknown until the next reconcile exchange consumes it.
pub(crate) fn task_protocol_conformance_commit_uncertainty_marker_digest(
    value: &TaskProtocolConformanceExchangeObservation,
) -> Result<String> {
    let remote_reference = value.remote_reference_digest.as_deref().ok_or_else(|| {
        anyhow::anyhow!("task-protocol commit uncertainty lacks remote reference")
    })?;
    let remote_sequence = value
        .remote_sequence
        .ok_or_else(|| anyhow::anyhow!("task-protocol commit uncertainty lacks remote sequence"))?;
    let digest_fields = [
        value.command_digest.as_str(),
        value.outbox_operation_digest.as_str(),
        value.route_authorization_digest.as_str(),
        value.synthetic_executor_digest.as_str(),
        value.fence_digest.as_str(),
        value.request_digest.as_str(),
        remote_reference,
    ];
    for field in &digest_fields {
        decode_digest(field)?;
    }
    let sequence_bytes = remote_sequence.to_be_bytes();
    let mut digest = Sha256::new();
    digest.update(TASK_PROTOCOL_CONFORMANCE_COMMIT_UNCERTAINTY_DOMAIN);
    for field in digest_fields
        .into_iter()
        .map(str::as_bytes)
        .chain(std::iter::once(sequence_bytes.as_slice()))
    {
        digest.update((field.len() as u64).to_be_bytes());
        digest.update(field);
    }
    Ok(hex::encode(digest.finalize()))
}

pub(crate) fn task_protocol_conformance_capability_fixture_digest(
    task_protocol_profile_digest: &str,
    fixture_catalog_digest: &str,
    capability_id: &str,
    capability_revision: u64,
    exchange_ordinals: &[u64],
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        task_protocol_profile_digest: &'a str,
        fixture_catalog_digest: &'a str,
        capability_id: &'a str,
        capability_revision: u64,
        exchange_ordinals: &'a [u64],
    }
    domain_digest(
        CAPABILITY_FIXTURE_DOMAIN,
        &Material {
            task_protocol_profile_digest,
            fixture_catalog_digest,
            capability_id,
            capability_revision,
            exchange_ordinals,
        },
    )
}

pub(crate) fn task_protocol_conformance_exchange_inventory_digest(
    exchanges: &[TaskProtocolConformanceExchangeObservation],
) -> Result<String> {
    domain_digest(EXCHANGE_INVENTORY_DOMAIN, exchanges)
}

pub(crate) fn task_protocol_conformance_delivery_inventory_digest(
    exchanges: &[TaskProtocolConformanceExchangeObservation],
) -> Result<String> {
    let roots: Vec<&str> = exchanges
        .iter()
        .map(|value| value.delivery_attempt_digest.as_str())
        .collect();
    domain_digest(DELIVERY_INVENTORY_DOMAIN, &roots)
}

pub(crate) fn task_protocol_conformance_task_observation_root(
    exchanges: &[TaskProtocolConformanceExchangeObservation],
    capabilities: &[TaskProtocolConformanceCapabilityObservation],
    cleanup: &TaskProtocolConformanceCleanupEvidence,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        exchanges: &'a [TaskProtocolConformanceExchangeObservation],
        capabilities: &'a [TaskProtocolConformanceCapabilityObservation],
        cleanup: &'a TaskProtocolConformanceCleanupEvidence,
    }
    domain_digest(
        TASK_OBSERVATION_DOMAIN,
        &Material {
            exchanges,
            capabilities,
            cleanup,
        },
    )
}

pub(crate) fn task_protocol_conformance_capability_assertion_inventory_digest(
    capability_id: &str,
    capability_revision: u64,
    status: &str,
    test_case_id: &str,
    fixture_digest: &str,
    exchange_ordinals: &[u64],
    exchange_inventory_digest: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        capability_id: &'a str,
        capability_revision: u64,
        status: &'a str,
        test_case_id: &'a str,
        fixture_digest: &'a str,
        exchange_ordinals: &'a [u64],
        exchange_inventory_digest: &'a str,
    }
    domain_digest(
        CAPABILITY_ASSERTION_INVENTORY_DOMAIN,
        &Material {
            capability_id,
            capability_revision,
            status,
            test_case_id,
            fixture_digest,
            exchange_ordinals,
            exchange_inventory_digest,
        },
    )
}

pub(crate) fn derive_task_protocol_conformance_synthetic_subjects(
    registry_release: &ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots,
    task_protocol_profile_digest: &str,
    fixture_catalog_digest: &str,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceSyntheticSubjects> {
    let lane = subject(
        "fixture_lane",
        TASK_PROTOCOL_CONFORMANCE_FIXTURE_LANE_ID,
        FIXTURE_LANE_SUBJECT_DOMAIN,
        registry_release,
        task_protocol_profile_digest,
        fixture_catalog_digest,
    )?;
    let executor = subject(
        "fixture_executor",
        TASK_PROTOCOL_CONFORMANCE_FIXTURE_EXECUTOR_ID,
        FIXTURE_EXECUTOR_SUBJECT_DOMAIN,
        registry_release,
        task_protocol_profile_digest,
        fixture_catalog_digest,
    )?;
    Ok(
        ExternalPoolAdapterTaskProtocolConformanceSyntheticSubjects {
            fixture_lane: lane,
            fixture_executor: executor,
        },
    )
}

#[derive(Serialize)]
struct SyntheticSubjectMaterial<'a> {
    subject_kind: &'a str,
    subject_id: &'a str,
    registry_release_id: &'a str,
    registry_release_digest: &'a str,
    registry_release_material_digest: &'a str,
    task_protocol_profile_id: &'static str,
    task_protocol_profile_revision: u64,
    task_protocol_profile_digest: &'a str,
    fixture_catalog_id: &'static str,
    fixture_catalog_revision: u64,
    fixture_catalog_digest: &'a str,
    authority_status: &'static str,
}

fn subject(
    subject_kind: &str,
    subject_id: &str,
    domain: &[u8],
    release: &ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots,
    profile_digest: &str,
    catalog_digest: &str,
) -> Result<ExternalPoolAdapterTaskProtocolConformanceSyntheticSubject> {
    let material = SyntheticSubjectMaterial {
        subject_kind,
        subject_id,
        registry_release_id: &release.registry_release_id,
        registry_release_digest: &release.registry_release_digest,
        registry_release_material_digest: &release.registry_release_material_digest,
        task_protocol_profile_id: TASK_PROTOCOL_CONFORMANCE_PROFILE_ID,
        task_protocol_profile_revision: TASK_PROTOCOL_CONFORMANCE_PROFILE_REVISION,
        task_protocol_profile_digest: profile_digest,
        fixture_catalog_id: TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_ID,
        fixture_catalog_revision: TASK_PROTOCOL_CONFORMANCE_FIXTURE_CATALOG_REVISION,
        fixture_catalog_digest: catalog_digest,
        authority_status: TASK_PROTOCOL_CONFORMANCE_NON_PRODUCTION_AUTHORITY,
    };
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(canonical_json(&material)?.as_bytes());
    Ok(ExternalPoolAdapterTaskProtocolConformanceSyntheticSubject {
        subject_kind: subject_kind.into(),
        subject_id: subject_id.into(),
        subject_digest: hex::encode(digest.finalize()),
        authority_status: TASK_PROTOCOL_CONFORMANCE_NON_PRODUCTION_AUTHORITY.into(),
    })
}

fn receipt_digest<T: Serialize>(
    value: &T,
    digest_field: &str,
    domain: &[u8],
    kind: &str,
) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{kind} must be an object"))?
        .clone();
    if projection
        .insert(
            digest_field.into(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("{kind} lacks its digest field")
    }
    Ok((canonical_json(value)?, domain_digest(domain, &projection)?))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json(value)?.as_bytes());
    Ok(hex::encode(digest.finalize()))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(
        value,
        TASK_PROTOCOL_CONFORMANCE_MAX_RECEIPT_JSON_BYTES,
    )
    .map(|item| item.0)
}

fn decode_digest(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("task-protocol conformance digest is not lowercase SHA-256")
    }
    let bytes = hex::decode(value)?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("task-protocol conformance digest length changed"))
}
