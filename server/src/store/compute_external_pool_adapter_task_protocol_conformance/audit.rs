use anyhow::{bail, Result};
use rusqlite::{named_params, Connection};

use crate::compute_federation::external_pool_adapter_task_protocol_conformance::*;

use super::{persistence::canonical_json, types::*};

pub(super) fn audit_run(
    conn: &Connection,
    stored: StoredTaskProtocolConformanceRun,
) -> Result<StoredTaskProtocolConformanceRun> {
    validate_task_protocol_conformance_run_receipt(&stored.receipt)?;
    let (canonical, digest) =
        canonical_task_protocol_conformance_run_receipt_json_and_digest(&stored.receipt)?;
    if canonical != stored.receipt_json || digest != stored.receipt.run_receipt_digest {
        bail!("task-protocol conformance run JSON is not canonical")
    }
    validate_run_private(&stored)?;
    audit_run_projection(conn, &stored)?;
    Ok(stored)
}

pub(super) fn audit_revocation(
    conn: &Connection,
    stored: StoredTaskProtocolConformanceRevocation,
) -> Result<StoredTaskProtocolConformanceRevocation> {
    validate_task_protocol_conformance_revocation_receipt(&stored.receipt)?;
    let (canonical, digest) =
        canonical_task_protocol_conformance_revocation_receipt_json_and_digest(&stored.receipt)?;
    if canonical != stored.receipt_json || digest != stored.receipt.revocation_receipt_digest {
        bail!("task-protocol conformance revocation JSON is not canonical")
    }
    validate_revocation_private(&stored)?;
    audit_revocation_projection(conn, &stored)?;
    Ok(stored)
}

fn validate_run_private(stored: &StoredTaskProtocolConformanceRun) -> Result<()> {
    private_identifier(&stored.recorded_by_admin_user_id)?;
    private_identifier(&stored.idempotency_key)?;
    let expected_scope = format!(
        "v272:task-protocol-conformance:create:{}",
        stored.recorded_by_admin_user_id
    );
    if stored.idempotency_scope != expected_scope
        || stored.confirmation != TASK_PROTOCOL_CONFORMANCE_CONFIRMATION
        || !digest(&stored.runtime_custody_epoch_digest)
        || !digest(&stored.process_hmac_seal)
        || !digest(&stored.receipt_integrity_digest)
        || task_protocol_conformance_receipt_integrity_digest(
            &stored.receipt.run_receipt_digest,
            &stored.runtime_custody_epoch_digest,
            &stored.process_hmac_seal,
        )? != stored.receipt_integrity_digest
    {
        bail!("task-protocol conformance private run projection is invalid")
    }
    Ok(())
}

fn validate_revocation_private(stored: &StoredTaskProtocolConformanceRevocation) -> Result<()> {
    private_identifier(&stored.revoked_by_admin_user_id)?;
    private_identifier(&stored.idempotency_key)?;
    let expected_scope = format!(
        "v272:task-protocol-conformance:revoke:{}",
        stored.revoked_by_admin_user_id
    );
    if stored.idempotency_scope != expected_scope
        || stored.confirmation != TASK_PROTOCOL_CONFORMANCE_REVOCATION_CONFIRMATION
    {
        bail!("task-protocol conformance private revocation projection is invalid")
    }
    Ok(())
}

fn audit_run_projection(
    conn: &Connection,
    stored: &StoredTaskProtocolConformanceRun,
) -> Result<()> {
    let receipt = &stored.receipt;
    let r = &receipt.run;
    let release = &r.registry_release;
    let vulnerability = &r.vulnerability_reattestation;
    let sandbox = &r.sandbox_reattestation;
    let key = &r.sandbox_verifier_key;
    let compatibility = &r.runtime_compatibility;
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts
          WHERE run_receipt_id=:run_id AND run_receipt_schema=:schema
            AND run_receipt_digest=:run_digest AND run_material_digest=:material_digest
            AND run_receipt_json=:json AND canonicalization=:canonicalization
            AND digest_algorithm=:algorithm AND registry_release_id=:release_id
            AND registry_release_digest=:release_digest
            AND registry_release_material_digest=:release_material
            AND admission_id=:admission_id AND admission_digest=:admission_digest
            AND package_receipt_id=:package_id AND package_receipt_digest=:package_digest
            AND package_material_digest=:package_material AND source_receipt_id=:source_id
            AND source_receipt_digest=:source_digest AND adapter_id=:adapter_id
            AND release_version=:release_version AND route_kind=:route_kind
            AND implementation_digest=:implementation
            AND declared_implementation_sha256=:declared_implementation
            AND entrypoint_path=:entrypoint_path AND entrypoint_sha256=:entrypoint_sha
            AND entrypoint_size_bytes=:entrypoint_size
            AND installation_content_digest=:installation_content
            AND capability_set_digest=:capability_set
            AND vulnerability_reattestation_receipt_id=:vulnerability_id
            AND vulnerability_reattestation_receipt_digest=:vulnerability_digest
            AND vulnerability_reattestation_material_digest=:vulnerability_material
            AND vulnerability_intelligence_snapshot_digest=:intelligence_snapshot
            AND vulnerability_intelligence_expires_at=:intelligence_expires
            AND sandbox_reattestation_receipt_id=:sandbox_id
            AND sandbox_reattestation_receipt_digest=:sandbox_digest
            AND sandbox_reattestation_material_digest=:sandbox_material
            AND sandbox_policy_id=:sandbox_policy AND sandbox_test_plan_digest=:sandbox_test_plan
            AND sandbox_observation_inventory_digest=:sandbox_observations
            AND sandbox_report_expires_at=:sandbox_expires
            AND sandbox_verifier_key_record_id=:key_record_id
            AND sandbox_verifier_key_record_digest=:key_record_digest
            AND sandbox_verifier_key_id=:key_id
            AND sandbox_verifier_operator=:key_operator
            AND sandbox_verifier_product=:key_product
            AND runtime_compatibility_verification_receipt_id=:verification_id
            AND runtime_compatibility_verification_receipt_digest=:verification_digest
            AND runtime_compatibility_verification_material_digest=:verification_material
            AND runtime_compatibility_run_observation_id=:observation_id
            AND runtime_compatibility_run_observation_digest=:observation_digest
            AND runtime_compatibility_run_observation_material_digest=:observation_material
            AND runtime_compatibility_runner_execution_id=:runner_execution_id
            AND runtime_compatibility_profile_id=:runtime_profile_id
            AND runtime_compatibility_profile_revision=:runtime_profile_revision
            AND runtime_compatibility_profile_digest=:runtime_profile_digest
            AND runtime_compatibility_runner_policy_digest=:runner_policy_digest
            AND runtime_compatibility_fixture_catalog_digest=:runtime_fixture_catalog_digest
            AND supervisor_session_policy_digest=:session_policy_digest
            AND source_capsule_sha256=:source_capsule_sha
            AND source_capsule_size_bytes=:source_capsule_size
            AND launch_image_sha256=:launch_image_sha AND launch_image_size_bytes=:launch_image_size
            AND runtime_compatibility_public_fixture_delivery_root=:runtime_public_delivery_root
            AND runtime_compatibility_expires_at=:runtime_expires
            AND task_protocol_profile_id=:task_profile_id
            AND task_protocol_profile_revision=:task_profile_revision
            AND task_protocol_profile_digest=:task_profile_digest
            AND fixture_catalog_id=:fixture_catalog_id
            AND fixture_catalog_revision=:fixture_catalog_revision
            AND fixture_catalog_digest=:fixture_catalog_digest AND run_nonce_digest=:run_nonce
            AND public_fixture_delivery_root=:public_delivery_root
            AND session_root_digests_json=:session_roots_json
            AND session_roots_digest=:session_roots_digest
            AND session_transcript_digest=:session_transcript_digest
            AND delivery_inventory_digest=:delivery_inventory_digest
            AND exchange_inventory_digest=:exchange_inventory_digest
            AND task_observation_root=:task_observation_root
            AND synthetic_fixture_lane_id=:lane_id AND synthetic_fixture_lane_digest=:lane_digest
            AND synthetic_fixture_executor_id=:executor_id
            AND synthetic_fixture_executor_digest=:executor_digest
            AND exchange_count=:exchange_count AND capability_count=:capability_count
            AND duration_ms=:duration_ms AND cleanup_json=:cleanup_json AND sequence=:sequence
            AND predecessor_run_receipt_id IS :predecessor_id
            AND predecessor_run_receipt_digest IS :predecessor_digest
            AND run_started_at=:run_started_at AND run_completed_at=:run_completed_at
            AND post_cleanup_checked_at=:checked_at AND expires_at=:expires_at
            AND recorded_at=:recorded_at AND evidence_scope=:evidence_scope
            AND receipt_status=:receipt_status
            AND non_production_authority_status=:authority_status
            AND effects_json=:effects_json AND readiness_json=:readiness_json
            AND recorded_by_admin_user_id=:admin_id AND idempotency_scope=:idempotency_scope
            AND idempotency_key=:idempotency_key AND confirmation=:confirmation
            AND runtime_custody_epoch_digest=:custody_epoch_digest
            AND process_hmac_seal=:process_hmac_seal
            AND receipt_integrity_digest=:receipt_integrity_digest)",
        named_params! {
            ":run_id": receipt.run_receipt_id,
            ":schema": receipt.schema,
            ":run_digest": receipt.run_receipt_digest,
            ":material_digest": receipt.run_material_digest,
            ":json": stored.receipt_json,
            ":canonicalization": receipt.canonicalization,
            ":algorithm": receipt.digest_algorithm,
            ":release_id": release.registry_release_id,
            ":release_digest": release.registry_release_digest,
            ":release_material": release.registry_release_material_digest,
            ":admission_id": release.admission_id,
            ":admission_digest": release.admission_digest,
            ":package_id": release.package_receipt_id,
            ":package_digest": release.package_receipt_digest,
            ":package_material": release.package_material_digest,
            ":source_id": release.source_receipt_id,
            ":source_digest": release.source_receipt_digest,
            ":adapter_id": release.adapter_id,
            ":release_version": release.release_version,
            ":route_kind": release.route_kind,
            ":implementation": release.implementation_digest,
            ":declared_implementation": release.declared_implementation_sha256,
            ":entrypoint_path": release.entrypoint_path,
            ":entrypoint_sha": release.entrypoint_sha256,
            ":entrypoint_size": i64::try_from(release.entrypoint_size_bytes)?,
            ":installation_content": release.installation_content_digest,
            ":capability_set": release.capability_set_digest,
            ":vulnerability_id": vulnerability.reattestation_receipt_id,
            ":vulnerability_digest": vulnerability.reattestation_receipt_digest,
            ":vulnerability_material": vulnerability.reattestation_material_digest,
            ":intelligence_snapshot": vulnerability.intelligence_snapshot_digest,
            ":intelligence_expires": vulnerability.intelligence_expires_at,
            ":sandbox_id": sandbox.reattestation_receipt_id,
            ":sandbox_digest": sandbox.reattestation_receipt_digest,
            ":sandbox_material": sandbox.reattestation_material_digest,
            ":sandbox_policy": sandbox.sandbox_policy_id,
            ":sandbox_test_plan": sandbox.test_plan_digest,
            ":sandbox_observations": sandbox.observation_inventory_digest,
            ":sandbox_expires": sandbox.report_expires_at,
            ":key_record_id": key.key_record_id,
            ":key_record_digest": key.key_record_digest,
            ":key_id": key.key_id,
            ":key_operator": key.verifier_operator,
            ":key_product": key.verifier_product,
            ":verification_id": compatibility.verification_receipt_id,
            ":verification_digest": compatibility.verification_receipt_digest,
            ":verification_material": compatibility.verification_material_digest,
            ":observation_id": compatibility.run_observation_id,
            ":observation_digest": compatibility.run_observation_digest,
            ":observation_material": compatibility.run_observation_material_digest,
            ":runner_execution_id": compatibility.runner_execution_id,
            ":runtime_profile_id": compatibility.profile_id,
            ":runtime_profile_revision": i64::try_from(compatibility.profile_revision)?,
            ":runtime_profile_digest": compatibility.profile_digest,
            ":runner_policy_digest": compatibility.runner_policy_digest,
            ":runtime_fixture_catalog_digest": compatibility.fixture_catalog_digest,
            ":session_policy_digest": compatibility.supervisor_session_policy_digest,
            ":source_capsule_sha": compatibility.source_capsule_sha256,
            ":source_capsule_size": i64::try_from(compatibility.source_capsule_size_bytes)?,
            ":launch_image_sha": compatibility.launch_image_sha256,
            ":launch_image_size": i64::try_from(compatibility.launch_image_size_bytes)?,
            ":runtime_public_delivery_root": compatibility.public_fixture_delivery_root,
            ":runtime_expires": compatibility.expires_at,
            ":task_profile_id": r.task_protocol_profile_id,
            ":task_profile_revision": i64::try_from(r.task_protocol_profile_revision)?,
            ":task_profile_digest": r.task_protocol_profile_digest,
            ":fixture_catalog_id": r.fixture_catalog_id,
            ":fixture_catalog_revision": i64::try_from(r.fixture_catalog_revision)?,
            ":fixture_catalog_digest": r.fixture_catalog_digest,
            ":run_nonce": r.run_nonce_digest,
            ":public_delivery_root": r.public_fixture_delivery_root,
            ":session_roots_json": canonical_json(&r.session_root_digests)?,
            ":session_roots_digest": r.session_roots_digest,
            ":session_transcript_digest": r.session_transcript_digest,
            ":delivery_inventory_digest": r.delivery_inventory_digest,
            ":exchange_inventory_digest": r.exchange_inventory_digest,
            ":task_observation_root": r.task_observation_root,
            ":lane_id": r.synthetic_subjects.fixture_lane.subject_id,
            ":lane_digest": r.synthetic_subjects.fixture_lane.subject_digest,
            ":executor_id": r.synthetic_subjects.fixture_executor.subject_id,
            ":executor_digest": r.synthetic_subjects.fixture_executor.subject_digest,
            ":exchange_count": i64::try_from(r.exchanges.len())?,
            ":capability_count": i64::try_from(r.capabilities.len())?,
            ":duration_ms": i64::try_from(r.duration_ms)?,
            ":cleanup_json": canonical_json(&r.cleanup)?,
            ":sequence": i64::try_from(r.sequence)?,
            ":predecessor_id": r.predecessor_run_receipt_id,
            ":predecessor_digest": r.predecessor_run_receipt_digest,
            ":run_started_at": r.run_started_at,
            ":run_completed_at": r.run_completed_at,
            ":checked_at": r.post_cleanup_checked_at,
            ":expires_at": r.expires_at,
            ":recorded_at": r.recorded_at,
            ":evidence_scope": r.evidence_scope,
            ":receipt_status": r.receipt_status,
            ":authority_status": r.non_production_authority_status,
            ":effects_json": canonical_json(&r.effects)?,
            ":readiness_json": canonical_json(&r.readiness)?,
            ":admin_id": stored.recorded_by_admin_user_id,
            ":idempotency_scope": stored.idempotency_scope,
            ":idempotency_key": stored.idempotency_key,
            ":confirmation": stored.confirmation,
            ":custody_epoch_digest": stored.runtime_custody_epoch_digest,
            ":process_hmac_seal": stored.process_hmac_seal,
            ":receipt_integrity_digest": stored.receipt_integrity_digest,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("task-protocol conformance SQL run projection is not exact")
    }
    Ok(())
}

fn audit_revocation_projection(
    conn: &Connection,
    stored: &StoredTaskProtocolConformanceRevocation,
) -> Result<()> {
    let receipt = &stored.receipt;
    let r = &receipt.revocation;
    let exact: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1
           FROM compute_external_pool_adapter_task_protocol_conformance_revocations
          WHERE revocation_receipt_id=:id AND revocation_receipt_schema=:schema
            AND revocation_receipt_digest=:digest AND revocation_material_digest=:material
            AND revocation_receipt_json=:json AND canonicalization=:canonicalization
            AND digest_algorithm=:algorithm AND run_receipt_id=:run_id
            AND run_receipt_digest=:run_digest AND registry_release_id=:release_id
            AND registry_release_digest=:release_digest AND reason=:reason
            AND revoked_at=:revoked_at AND recorded_at=:recorded_at
            AND revocation_status=:status AND effects_json=:effects
            AND readiness_json=:readiness AND revoked_by_admin_user_id=:admin_id
            AND idempotency_scope=:scope AND idempotency_key=:key
            AND confirmation=:confirmation)",
        named_params! {
            ":id": receipt.revocation_receipt_id,
            ":schema": receipt.schema,
            ":digest": receipt.revocation_receipt_digest,
            ":material": receipt.revocation_material_digest,
            ":json": stored.receipt_json,
            ":canonicalization": receipt.canonicalization,
            ":algorithm": receipt.digest_algorithm,
            ":run_id": r.run_receipt_id,
            ":run_digest": r.run_receipt_digest,
            ":release_id": r.registry_release_id,
            ":release_digest": r.registry_release_digest,
            ":reason": r.reason,
            ":revoked_at": r.revoked_at,
            ":recorded_at": r.recorded_at,
            ":status": r.revocation_status,
            ":effects": canonical_json(&r.effects)?,
            ":readiness": canonical_json(&r.readiness)?,
            ":admin_id": stored.revoked_by_admin_user_id,
            ":scope": stored.idempotency_scope,
            ":key": stored.idempotency_key,
            ":confirmation": stored.confirmation,
        },
        |row| row.get(0),
    )?;
    if !exact {
        bail!("task-protocol conformance SQL revocation projection is not exact")
    }
    Ok(())
}

fn private_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("task-protocol conformance private identifier is invalid")
    }
    Ok(())
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
