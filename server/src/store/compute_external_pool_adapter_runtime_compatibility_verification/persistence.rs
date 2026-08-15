use anyhow::Result;
use rusqlite::{named_params, Transaction};

use crate::compute_federation::external_pool_adapter_runtime_compatibility_verification::*;

pub(super) fn insert_challenge(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
) -> Result<()> {
    let c = &receipt.challenge;
    let release = &c.registry_release;
    let item = &release.release;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_runtime_compatibility_verification_challenges(
          challenge_id,challenge_schema,challenge_digest,challenge_material_digest,challenge_json,
          canonicalization,digest_algorithm,challenge_nonce_digest,issued_at,expires_at,
          registry_release_json,registry_release_id,registry_release_digest,registry_release_material_digest,
          adapter_id,release_version,installation_content_digest,runtime_kind,entrypoint_path,
          entrypoint_sha256,entrypoint_size_bytes,profile_id,profile_revision,profile_digest,
          supervisor_session_policy_digest,source_capsule_policy_digest,runner_policy_digest,
          fixture_catalog_digest,fixture_resources_json,sandbox_verifier_key_record_id,
          sandbox_verifier_key_record_digest,sandbox_verifier_key_id,sandbox_verifier_operator,
          sandbox_verifier_product,signature_algorithm,sequence,predecessor_verification_receipt_id,
          predecessor_verification_receipt_digest,created_by_admin_user_id,confirmation,
          idempotency_scope,idempotency_key,recorded_at
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:nonce_digest,:issued,
          :expires,:release_json,:release_id,:release_digest,:release_material,:adapter_id,
          :release_version,:installation,:runtime_kind,:entrypoint_path,:entrypoint_sha,
          :entrypoint_size,:profile_id,:profile_revision,:profile_digest,:session_policy,
          :source_policy,:runner_policy,:fixture_catalog,:fixtures,:key_record_id,:key_record_digest,
          :key_id,:operator,:product,:signature_algorithm,:sequence,:predecessor_id,
          :predecessor_digest,:actor,:confirmation,:scope,:key,:recorded_at)",
        named_params! {
            ":id": c.challenge_id, ":schema": receipt.schema,
            ":digest": receipt.challenge_digest, ":material": receipt.challenge_material_digest,
            ":json": runtime_compatibility_challenge_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":nonce_digest": c.challenge_nonce_digest, ":issued": c.issued_at,
            ":expires": c.expires_at,
            ":release_json": canonical_runtime_compatibility_verification_json(release)?,
            ":release_id": release.registry_release_id,
            ":release_digest": release.registry_release_digest,
            ":release_material": release.registry_release_material_digest,
            ":adapter_id": item.adapter_id, ":release_version": item.release_version,
            ":installation": item.installation_content_digest, ":runtime_kind": c.runtime_kind,
            ":entrypoint_path": c.entrypoint_path, ":entrypoint_sha": c.entrypoint_sha256,
            ":entrypoint_size": i64::try_from(c.entrypoint_size_bytes)?,
            ":profile_id": c.profile_id, ":profile_revision": i64::try_from(c.profile_revision)?,
            ":profile_digest": c.profile_digest,
            ":session_policy": c.supervisor_session_policy.policy_digest,
            ":source_policy": c.source_capsule_policy.policy_digest,
            ":runner_policy": c.runner_policy.policy_digest,
            ":fixture_catalog": c.fixture_catalog.policy_digest,
            ":fixtures": canonical_runtime_compatibility_verification_json(&c.fixture_resources)?,
            ":key_record_id": c.sandbox_verifier_key_record_id,
            ":key_record_digest": c.sandbox_verifier_key_record_digest,
            ":key_id": c.sandbox_verifier_key_id, ":operator": c.sandbox_verifier_operator,
            ":product": c.sandbox_verifier_product, ":signature_algorithm": c.signature_algorithm,
            ":sequence": i64::try_from(c.sequence)?,
            ":predecessor_id": c.predecessor_verification_receipt_id,
            ":predecessor_digest": c.predecessor_verification_receipt_digest,
            ":actor": c.created_by_admin_user_id, ":confirmation": c.confirmation,
            ":scope": c.idempotency_scope, ":key": c.idempotency_key,
            ":recorded_at": c.recorded_at,
        },
    )?;
    Ok(())
}

pub(super) fn insert_run_observation(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<()> {
    let o = &receipt.observation;
    let release = &o.registry_release;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_runtime_compatibility_verification_run_observations(
          run_observation_id,run_observation_schema,run_observation_digest,
          run_observation_material_digest,run_observation_json,canonicalization,digest_algorithm,
          runner_execution_id,challenge_id,challenge_digest,challenge_nonce_digest,
          registry_release_id,registry_release_digest,registry_release_material_digest,
          installation_content_digest,profile_id,profile_revision,profile_digest,runner_policy_digest,
          fixture_catalog_digest,source_capsule_sha256,source_capsule_size_bytes,
          source_capsule_policy_digest,launch_image_sha256,launch_image_size_bytes,
          public_fixture_delivery_root,fixture_resources_json,observations_json,no_work_json,
          run_started_at,run_completed_at,recorded_at,child_network_attempt_count,
          upstream_connect_attempt_count,write_outside_ephemeral_count,additional_process_attempt_count,
          policy_violation_count,observation_status,effects_json,readiness_json
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:execution_id,
          :challenge_id,:challenge_digest,:nonce_digest,:release_id,:release_digest,
          :release_material,:installation,:profile_id,:profile_revision,:profile_digest,
          :runner_policy,:fixture_catalog,:source_sha,:source_size,:source_policy,:launch_sha,
          :launch_size,:delivery_root,:fixtures,:observations,:no_work,:started,:completed,
          :recorded,:network_count,:upstream_count,:write_count,:process_count,:policy_count,
          :status,:effects,:readiness)",
        named_params! {
            ":id": receipt.run_observation_id, ":schema": receipt.schema,
            ":digest": receipt.run_observation_digest,
            ":material": receipt.run_observation_material_digest,
            ":json": runtime_compatibility_observation_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":execution_id": o.runner_execution_id, ":challenge_id": o.challenge_id,
            ":challenge_digest": o.challenge_digest, ":nonce_digest": o.challenge_nonce_digest,
            ":release_id": release.registry_release_id,
            ":release_digest": release.registry_release_digest,
            ":release_material": release.registry_release_material_digest,
            ":installation": release.release.installation_content_digest,
            ":profile_id": o.profile_id, ":profile_revision": i64::try_from(o.profile_revision)?,
            ":profile_digest": o.profile_digest, ":runner_policy": o.runner_policy_digest,
            ":fixture_catalog": o.fixture_catalog_digest, ":source_sha": o.source_capsule_sha256,
            ":source_size": i64::try_from(o.source_capsule_size_bytes)?,
            ":source_policy": o.source_capsule_policy_digest, ":launch_sha": o.launch_image_sha256,
            ":launch_size": i64::try_from(o.launch_image_size_bytes)?,
            ":delivery_root": o.public_fixture_delivery_root,
            ":fixtures": canonical_runtime_compatibility_verification_json(&o.fixture_resources)?,
            ":observations": canonical_runtime_compatibility_verification_json(&o.observations)?,
            ":no_work": canonical_runtime_compatibility_verification_json(&o.no_work)?,
            ":started": o.run_started_at, ":completed": o.run_completed_at,
            ":recorded": o.recorded_at,
            ":network_count": i64::try_from(o.child_network_attempt_count)?,
            ":upstream_count": i64::try_from(o.upstream_connect_attempt_count)?,
            ":write_count": i64::try_from(o.write_outside_ephemeral_count)?,
            ":process_count": i64::try_from(o.additional_process_attempt_count)?,
            ":policy_count": i64::try_from(o.policy_violation_count)?,
            ":status": o.observation_status,
            ":effects": canonical_runtime_compatibility_verification_json(&o.effects)?,
            ":readiness": canonical_runtime_compatibility_verification_json(&o.readiness)?,
        },
    )?;
    Ok(())
}

pub(super) fn insert_verification(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
) -> Result<()> {
    let v = &receipt.verification;
    let release = &v.registry_release;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_runtime_compatibility_verification_receipts(
          verification_receipt_id,verification_receipt_schema,verification_receipt_digest,
          verification_material_digest,verification_receipt_json,canonicalization,digest_algorithm,
          runner_execution_id,challenge_id,challenge_digest,run_observation_id,run_observation_digest,
          run_observation_material_digest,registry_release_id,registry_release_digest,
          registry_release_material_digest,installation_content_digest,profile_id,profile_revision,
          profile_digest,runner_policy_digest,fixture_catalog_digest,public_fixture_delivery_root,
          sandbox_verifier_key_record_id,sandbox_verifier_key_record_digest,sandbox_verifier_key_id,
          sandbox_verifier_operator,sandbox_verifier_product,signature_algorithm,
          signature_message_digest,signature_base64,signature_digest,sequence,
          predecessor_verification_receipt_id,predecessor_verification_receipt_digest,
          verified_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,verified_at,
          recorded_at,expires_at,evidence_scope,receipt_status,effects_json,readiness_json
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:execution_id,
          :challenge_id,:challenge_digest,:observation_id,:observation_digest,:observation_material,
          :release_id,:release_digest,:release_material,:installation,:profile_id,:profile_revision,
          :profile_digest,:runner_policy,:fixture_catalog,:delivery_root,:key_record_id,
          :key_record_digest,:key_id,:operator,:product,:signature_algorithm,:message_digest,
          :signature,:signature_digest,:sequence,:predecessor_id,:predecessor_digest,:actor,
          :confirmation,:scope,:key,:verified,:recorded,:expires,:evidence_scope,:status,:effects,
          :readiness)",
        named_params! {
            ":id": receipt.verification_receipt_id, ":schema": receipt.schema,
            ":digest": receipt.verification_receipt_digest,
            ":material": receipt.verification_material_digest,
            ":json": runtime_compatibility_verification_receipt_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":execution_id": v.runner_execution_id, ":challenge_id": v.challenge_id,
            ":challenge_digest": v.challenge_digest, ":observation_id": v.run_observation_id,
            ":observation_digest": v.run_observation_digest,
            ":observation_material": v.run_observation_material_digest,
            ":release_id": release.registry_release_id,
            ":release_digest": release.registry_release_digest,
            ":release_material": release.registry_release_material_digest,
            ":installation": release.release.installation_content_digest,
            ":profile_id": v.profile_id, ":profile_revision": i64::try_from(v.profile_revision)?,
            ":profile_digest": v.profile_digest, ":runner_policy": v.runner_policy_digest,
            ":fixture_catalog": v.fixture_catalog_digest,
            ":delivery_root": v.public_fixture_delivery_root,
            ":key_record_id": v.sandbox_verifier_key_record_id,
            ":key_record_digest": v.sandbox_verifier_key_record_digest,
            ":key_id": v.sandbox_verifier_key_id, ":operator": v.sandbox_verifier_operator,
            ":product": v.sandbox_verifier_product, ":signature_algorithm": v.signature_algorithm,
            ":message_digest": v.signature_message_digest, ":signature": v.signature_base64,
            ":signature_digest": v.signature_digest, ":sequence": i64::try_from(v.sequence)?,
            ":predecessor_id": v.predecessor_verification_receipt_id,
            ":predecessor_digest": v.predecessor_verification_receipt_digest,
            ":actor": v.verified_by_admin_user_id, ":confirmation": v.confirmation,
            ":scope": v.idempotency_scope, ":key": v.idempotency_key,
            ":verified": v.verified_at, ":recorded": v.recorded_at, ":expires": v.expires_at,
            ":evidence_scope": v.evidence_scope, ":status": v.receipt_status,
            ":effects": canonical_runtime_compatibility_verification_json(&v.effects)?,
            ":readiness": canonical_runtime_compatibility_verification_json(&v.readiness)?,
        },
    )?;
    Ok(())
}

pub(super) fn insert_revocation(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt,
) -> Result<()> {
    let r = &receipt.revocation;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_runtime_compatibility_verification_revocations(
          revocation_receipt_id,revocation_receipt_schema,revocation_receipt_digest,
          revocation_material_digest,revocation_receipt_json,canonicalization,digest_algorithm,
          verification_receipt_id,verification_receipt_digest,registry_release_id,
          registry_release_digest,revoked_by_admin_user_id,reason,confirmation,idempotency_scope,
          idempotency_key,revoked_at,recorded_at,revocation_status,effects_json,readiness_json
        ) VALUES (:id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:verification_id,
          :verification_digest,:release_id,:release_digest,:actor,:reason,:confirmation,:scope,:key,
          :revoked,:recorded,:status,:effects,:readiness)",
        named_params! {
            ":id": receipt.revocation_receipt_id, ":schema": receipt.schema,
            ":digest": receipt.revocation_receipt_digest,
            ":material": receipt.revocation_material_digest,
            ":json": runtime_compatibility_revocation_receipt_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":verification_id": r.verification_receipt_id,
            ":verification_digest": r.verification_receipt_digest,
            ":release_id": r.registry_release_id, ":release_digest": r.registry_release_digest,
            ":actor": r.revoked_by_admin_user_id, ":reason": r.reason,
            ":confirmation": r.confirmation, ":scope": r.idempotency_scope,
            ":key": r.idempotency_key, ":revoked": r.revoked_at,
            ":recorded": r.recorded_at, ":status": r.revocation_status,
            ":effects": canonical_runtime_compatibility_verification_json(&r.effects)?,
            ":readiness": canonical_runtime_compatibility_verification_json(&r.readiness)?,
        },
    )?;
    Ok(())
}
