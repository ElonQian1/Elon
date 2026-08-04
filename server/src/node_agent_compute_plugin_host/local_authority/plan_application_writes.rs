use anyhow::{bail, Context, Result};
use rusqlite::{named_params, params, Transaction};
use serde::Serialize;

use super::{
    keyring_snapshot::PersistedComputePluginKeyringSnapshot,
    plan_application::{
        AuthorityPlanApplicationState, ComputePluginPlanApplicationReceipt,
        PreparedPlanApplicationRequest,
    },
    plan_application_projection::{ProjectedCandidateClosure, ProjectedPlanApplication},
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::AdmittedComputePluginInstallPlan,
    lifecycle::ComputePluginInventorySnapshot, signed_artifact_verification::jcs_sha256_hex,
};

const PLAN_APPLIED_EVENT_SCHEMA: &str = "elon.compute_plugin.plan_applied_event.v1";

#[derive(Serialize)]
struct PlanAppliedEvent<'a> {
    schema: &'static str,
    plan_id: &'a str,
    plan_digest: &'a str,
    application_request_digest: &'a str,
    admission_bindings_digest: &'a str,
    inventory_after_revision: i64,
    inventory_after_digest: &'a str,
    receipt_digest: &'a str,
    recorded_at_ms: i64,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn insert_plan_application(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    keyring: &PersistedComputePluginKeyringSnapshot,
    request: &PreparedPlanApplicationRequest,
    admitted: &AdmittedComputePluginInstallPlan,
    projected: &ProjectedPlanApplication,
    signed_plan_json: &str,
    signed_manifests_json: &str,
    admission_bindings_json: &str,
    admission_bindings_digest: &str,
    inventory_after_json: &str,
    inventory_after_digest: &str,
    application_state_revision: i64,
    authority_epoch: i64,
    download_bytes: i64,
    receipt_json: &str,
    receipt_digest: &str,
    applied_at_ms: i64,
    expires_at_ms: i64,
) -> Result<()> {
    let plan = admitted.plan();
    let inserted = transaction
        .execute(
            r#"INSERT INTO plan_applications (
            plan_id, plan_digest, application_request_digest,
            signed_plan_envelope_digest, signed_manifest_set_digest,
            signed_plan_json, signed_manifests_json, admission_bindings_json,
            admission_bindings_digest, expected_inventory_revision,
            expected_inventory_digest, application_inventory_revision,
            inventory_after_digest, inventory_after_json, application_state_revision,
            authority_epoch_at_apply, keyring_bundle_revision, publisher_keyring_revision,
            publisher_keyring_digest, control_keyring_revision, control_keyring_digest,
            control_signing_key_fingerprint, new_candidate_count, closed_candidate_count,
            download_count, download_bytes, applied_at_ms, expires_at_ms,
            receipt_json, receipt_digest
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
            ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30
        )"#,
            params![
                &plan.plan_id,
                admitted.plan_digest(),
                &request.application_request_digest,
                &request.signed_plan_envelope_digest,
                &request.signed_manifest_set_digest,
                signed_plan_json,
                signed_manifests_json,
                admission_bindings_json,
                admission_bindings_digest,
                plan.expected_inventory_revision,
                &plan.expected_inventory_digest,
                projected.inventory_after.inventory_revision,
                inventory_after_digest,
                inventory_after_json,
                application_state_revision,
                authority_epoch,
                keyring.bundle_revision(),
                keyring.publisher_binding().revision,
                &keyring.publisher_binding().digest,
                keyring.control_binding().revision,
                &keyring.control_binding().digest,
                admitted.control_signing_key_fingerprint(),
                i64::try_from(projected.candidates_to_create.len())?,
                i64::try_from(projected.candidates_to_release.len())?,
                i64::try_from(projected.downloads.len())?,
                download_bytes,
                applied_at_ms,
                expires_at_ms,
                receipt_json,
                receipt_digest,
            ],
        )
        .context("COMPUTE_PLUGIN_PLAN_APPLICATION_INSERT")?;
    if inserted != 1 || authority.inventory.inventory_revision != plan.expected_inventory_revision {
        bail!("COMPUTE_PLUGIN_PLAN_APPLICATION_INSERT_CAS");
    }
    Ok(())
}

pub(super) fn release_candidates(
    transaction: &Transaction<'_>,
    closures: &[ProjectedCandidateClosure],
    closing_plan_id: &str,
    closing_plan_digest: &str,
    applied_at_ms: i64,
) -> Result<()> {
    for closure in closures {
        transaction
            .execute(
                r#"UPDATE fetch_claims SET state = 'revoked', resolved_at_ms = ?1,
                resolution_reason = 'candidate_released_by_plan'
            WHERE candidate_token = ?2 AND state = 'prepared'"#,
                params![applied_at_ms, &closure.candidate_token],
            )
            .context("COMPUTE_PLUGIN_PLAN_FETCH_CLAIMS_REVOKE")?;
        transaction
            .execute(
                r#"UPDATE planned_downloads SET state = 'canceled', updated_at_ms = ?1
            WHERE candidate_token = ?2 AND state IN ('pending', 'downloading', 'failed')"#,
                params![applied_at_ms, &closure.candidate_token],
            )
            .context("COMPUTE_PLUGIN_PLAN_DOWNLOADS_CANCEL")?;
        let updated = transaction
            .execute(
                r#"UPDATE candidate_owners SET state = 'released', closed_at_ms = ?1,
                closed_by_plan_id = ?2, closed_by_plan_digest = ?3,
                close_reason = 'cancel_candidate'
            WHERE candidate_token = ?4 AND plugin_id = ?5 AND slot_ref = ?6
              AND candidate_generation = ?7 AND owner_plan_id = ?8
              AND owner_plan_digest = ?9 AND state = 'owned'"#,
                params![
                    applied_at_ms,
                    closing_plan_id,
                    closing_plan_digest,
                    &closure.candidate_token,
                    &closure.plugin_id,
                    &closure.slot_ref,
                    closure.candidate_generation,
                    &closure.owner_plan_id,
                    &closure.owner_plan_digest,
                ],
            )
            .context("COMPUTE_PLUGIN_PLAN_CANDIDATE_RELEASE")?;
        let unfinished = transaction
            .query_row(
                r#"SELECT COUNT(*) FROM planned_downloads
            WHERE candidate_token = ?1 AND state NOT IN ('complete', 'canceled')"#,
                [&closure.candidate_token],
                |row| row.get::<_, i64>(0),
            )
            .context("COMPUTE_PLUGIN_PLAN_CANDIDATE_DOWNLOADS_CHECK")?;
        if updated != 1 || unfinished != 0 {
            bail!("COMPUTE_PLUGIN_PLAN_CANDIDATE_RELEASE_CAS");
        }
    }
    Ok(())
}

pub(super) fn insert_candidates_and_downloads(
    transaction: &Transaction<'_>,
    projected: &ProjectedPlanApplication,
    plan_id: &str,
    plan_digest: &str,
    application_inventory_revision: i64,
    applied_at_ms: i64,
) -> Result<()> {
    for candidate in &projected.candidates_to_create {
        let release_json = serde_json::to_string(&candidate.release)
            .context("COMPUTE_PLUGIN_PLAN_CANDIDATE_RELEASE_JSON")?;
        transaction
            .execute(
                r#"INSERT INTO candidate_owners (
                candidate_token, plugin_id, slot_ref, candidate_generation, release_json,
                permission_grant_digest, owner_plan_id, owner_plan_digest,
                application_inventory_revision, state, created_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'owned', ?10)"#,
                params![
                    &candidate.candidate_token,
                    &candidate.plugin_id,
                    &candidate.slot_ref,
                    candidate.candidate_generation,
                    release_json,
                    &candidate.permission_grant_digest,
                    plan_id,
                    plan_digest,
                    application_inventory_revision,
                    applied_at_ms,
                ],
            )
            .context("COMPUTE_PLUGIN_PLAN_CANDIDATE_INSERT")?;
    }
    for download in &projected.downloads {
        transaction
            .execute(
                r#"INSERT INTO planned_downloads (
                plan_id, plan_digest, ordinal, item_index, candidate_token,
                artifact_kind, artifact_id, artifact_digest, source_ref, cache_class,
                part_relative_path, size_bytes, committed_offset, cursor_generation,
                state, created_at_ms, updated_at_ms
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, 0, 0, 'pending', ?13, ?13
            )"#,
                params![
                    plan_id,
                    plan_digest,
                    download.ordinal,
                    download.item_index,
                    &download.candidate_token,
                    &download.artifact_kind,
                    &download.artifact_id,
                    &download.artifact_digest,
                    &download.source_ref,
                    &download.cache_class,
                    &download.part_relative_path,
                    download.size_bytes,
                    applied_at_ms,
                ],
            )
            .context("COMPUTE_PLUGIN_PLAN_DOWNLOAD_INSERT")?;
    }
    Ok(())
}

pub(super) fn insert_event_and_seal(
    transaction: &Transaction<'_>,
    receipt: &ComputePluginPlanApplicationReceipt,
    receipt_digest: &str,
    applied_at_ms: i64,
) -> Result<()> {
    let event = PlanAppliedEvent {
        schema: PLAN_APPLIED_EVENT_SCHEMA,
        plan_id: &receipt.plan_id,
        plan_digest: &receipt.plan_digest,
        application_request_digest: &receipt.application_request_digest,
        admission_bindings_digest: &receipt.admission_bindings_digest,
        inventory_after_revision: receipt.inventory_after_revision,
        inventory_after_digest: &receipt.inventory_after_digest,
        receipt_digest,
        recorded_at_ms: applied_at_ms,
    };
    let event_digest = jcs_sha256_hex(&event)?;
    let payload_json =
        serde_json::to_string(&event).context("COMPUTE_PLUGIN_PLAN_APPLIED_EVENT_JSON")?;
    transaction
        .execute(
            r#"INSERT INTO plan_events (
            plan_id, plan_digest, event_index, event_type, event_digest,
            payload_json, recorded_at_ms
        ) VALUES (?1, ?2, 0, 'applied', ?3, ?4, ?5)"#,
            params![
                &receipt.plan_id,
                &receipt.plan_digest,
                event_digest,
                payload_json,
                applied_at_ms,
            ],
        )
        .context("COMPUTE_PLUGIN_PLAN_APPLIED_EVENT_INSERT")?;
    transaction
        .execute(
            r#"INSERT INTO plan_application_seals (
            plan_id, plan_digest, application_request_digest, receipt_digest, sealed_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            params![
                &receipt.plan_id,
                &receipt.plan_digest,
                &receipt.application_request_digest,
                receipt_digest,
                applied_at_ms,
            ],
        )
        .context("COMPUTE_PLUGIN_PLAN_APPLICATION_SEAL")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn update_authority_meta(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    inventory_after: &ComputePluginInventorySnapshot,
    inventory_after_json: &str,
    inventory_after_digest: &str,
    application_state_revision: i64,
    authority_epoch: i64,
    applied_at_ms: i64,
) -> Result<()> {
    let authorization_ref = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.authorization_ref.as_str());
    let authorization_revision = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.revision);
    let authorization_digest = authority
        .sharing_authorization
        .as_ref()
        .map(|binding| binding.digest.as_str());
    let updated = transaction
        .execute(
            r#"UPDATE authority_meta SET
            state_revision = :new_state_revision,
            inventory_revision = :new_inventory_revision,
            inventory_digest = :new_inventory_digest,
            inventory_json = :new_inventory_json,
            desired_policy_revision = :desired_policy_revision,
            sharing_enabled = :sharing_enabled,
            authority_epoch = :new_authority_epoch,
            trusted_time_high_water_ms = :trusted_now,
            clock_status = 'trusted',
            updated_at_ms = :trusted_now
        WHERE singleton = 1
          AND installation_id_digest = :installation_id_digest
          AND state_revision = :old_state_revision
          AND inventory_revision = :old_inventory_revision
          AND inventory_digest = :old_inventory_digest
          AND inventory_json = :old_inventory_json
          AND desired_policy_revision = :desired_policy_revision
          AND sharing_enabled = :sharing_enabled
          AND sharing_authorization_ref IS :authorization_ref
          AND sharing_authorization_revision IS :authorization_revision
          AND sharing_authorization_digest IS :authorization_digest
          AND node_profile_digest = :node_profile_digest
          AND manifest_catalog_revision = :manifest_catalog_revision
          AND target_id = :target_id
          AND host_api_protocol_id = :host_api_protocol_id
          AND host_api_revision = :host_api_revision
          AND active_bundle_revision = :bundle_revision
          AND publisher_keyring_revision = :publisher_revision
          AND publisher_keyring_digest = :publisher_digest
          AND control_keyring_revision = :control_revision
          AND control_keyring_digest = :control_digest
          AND authority_epoch = :old_authority_epoch
          AND process_owner_epoch = :process_owner_epoch
          AND trusted_time_high_water_ms = :old_trusted_time
          AND clock_status = 'trusted'"#,
            named_params! {
                ":new_state_revision": application_state_revision,
                ":new_inventory_revision": inventory_after.inventory_revision,
                ":new_inventory_digest": inventory_after_digest,
                ":new_inventory_json": inventory_after_json,
                ":desired_policy_revision": inventory_after.desired_policy_revision,
                ":sharing_enabled": if inventory_after.sharing_enabled { 1_i64 } else { 0_i64 },
                ":new_authority_epoch": authority_epoch,
                ":trusted_now": applied_at_ms,
                ":installation_id_digest": &authority.installation_id_digest,
                ":old_state_revision": authority.state_revision,
                ":old_inventory_revision": authority.inventory.inventory_revision,
                ":old_inventory_digest": &authority.inventory_digest,
                ":old_inventory_json": &authority.inventory_json,
                ":authorization_ref": authorization_ref,
                ":authorization_revision": authorization_revision,
                ":authorization_digest": authorization_digest,
                ":node_profile_digest": &authority.node_profile_digest,
                ":manifest_catalog_revision": authority.manifest_catalog_revision,
                ":target_id": &authority.target_id,
                ":host_api_protocol_id": &authority.host_api_protocol_id,
                ":host_api_revision": i64::from(authority.host_api_revision),
                ":bundle_revision": authority.keyring_bundle_revision,
                ":publisher_revision": authority.publisher_keyring.revision,
                ":publisher_digest": &authority.publisher_keyring.digest,
                ":control_revision": authority.control_keyring.revision,
                ":control_digest": &authority.control_keyring.digest,
                ":old_authority_epoch": authority.authority_epoch,
                ":process_owner_epoch": authority.process_owner_epoch,
                ":old_trusted_time": authority.trusted_time_high_water_ms,
            },
        )
        .context("COMPUTE_PLUGIN_PLAN_AUTHORITY_UPDATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_PLAN_AUTHORITY_CAS");
    }
    Ok(())
}
