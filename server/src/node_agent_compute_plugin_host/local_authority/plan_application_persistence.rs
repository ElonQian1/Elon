use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use super::{
    fetch_claim_revocation::revoke_for_plan_authority_epoch_advance,
    keyring_snapshot::PersistedComputePluginKeyringSnapshot,
    plan_application::{
        prepare_application_request, AuthorityPlanApplicationState,
        ComputePluginPlanApplicationDisposition, ComputePluginPlanApplicationReceipt,
        ComputePluginPlanApplicationResult, PreparedPlanApplicationRequest,
        PLAN_APPLICATION_RECEIPT_SCHEMA,
    },
    plan_application_projection::{
        PersistedAdmissionBindings, ProjectedPlanApplication, ADMISSION_BINDINGS_SCHEMA,
    },
    plan_application_replay_children::{
        read_created_candidates, read_downloads, read_released_candidates, restore_execution_plan,
        validate_replayed_children,
    },
    plan_application_writes::{
        insert_candidates_and_downloads, insert_event_and_seal, insert_plan_application,
        release_candidates, update_authority_meta,
    },
};
use crate::node_agent_compute_plugin_host::{
    identity::ComputePluginReleaseRef,
    install_plan::SignedComputePluginInstallPlan,
    install_plan_admission::AdmittedComputePluginInstallPlan,
    install_plan_admission_validation::is_identifier,
    install_plan_reauthorization::{manifest_binding_action_is_valid, manifest_release_for_item},
    lifecycle::{
        local_record_shape_is_valid, ComputePluginInventorySnapshot,
        COMPUTE_PLUGIN_INVENTORY_SCHEMA, MAX_COMPUTE_PLUGIN_INVENTORY_RECORDS,
    },
    manifest_validation::is_sha256,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPlanAppliedEvent {
    schema: String,
    plan_id: String,
    plan_digest: String,
    application_request_digest: String,
    admission_bindings_digest: String,
    inventory_after_revision: i64,
    inventory_after_digest: String,
    receipt_digest: String,
    recorded_at_ms: i64,
}

pub(super) fn replay_plan_application(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    application_request_digest: &str,
) -> Result<Option<ComputePluginPlanApplicationResult>> {
    let stored = transaction
        .query_row(
            r#"SELECT plan_digest, application_request_digest,
                signed_plan_envelope_digest, signed_manifest_set_digest,
                signed_plan_json, signed_manifests_json,
                admission_bindings_json, admission_bindings_digest,
                inventory_after_json, inventory_after_digest,
                expected_inventory_revision, expected_inventory_digest,
                application_inventory_revision, application_state_revision,
                authority_epoch_at_apply, keyring_bundle_revision,
                publisher_keyring_revision, publisher_keyring_digest,
                control_keyring_revision, control_keyring_digest,
                control_signing_key_fingerprint, new_candidate_count, closed_candidate_count,
                download_count, download_bytes, receipt_json, receipt_digest, applied_at_ms,
                expires_at_ms
            FROM plan_applications WHERE plan_id = ?1"#,
            [plan_id],
            |row| {
                Ok(StoredPlanApplication {
                    plan_digest: row.get(0)?,
                    application_request_digest: row.get(1)?,
                    signed_plan_envelope_digest: row.get(2)?,
                    signed_manifest_set_digest: row.get(3)?,
                    signed_plan_json: row.get(4)?,
                    signed_manifests_json: row.get(5)?,
                    admission_bindings_json: row.get(6)?,
                    admission_bindings_digest: row.get(7)?,
                    inventory_after_json: row.get(8)?,
                    inventory_after_digest: row.get(9)?,
                    expected_inventory_revision: row.get(10)?,
                    expected_inventory_digest: row.get(11)?,
                    application_inventory_revision: row.get(12)?,
                    application_state_revision: row.get(13)?,
                    authority_epoch_at_apply: row.get(14)?,
                    keyring_bundle_revision: row.get(15)?,
                    publisher_keyring_revision: row.get(16)?,
                    publisher_keyring_digest: row.get(17)?,
                    control_keyring_revision: row.get(18)?,
                    control_keyring_digest: row.get(19)?,
                    control_signing_key_fingerprint: row.get(20)?,
                    new_candidate_count: row.get(21)?,
                    closed_candidate_count: row.get(22)?,
                    download_count: row.get(23)?,
                    download_bytes: row.get(24)?,
                    receipt_json: row.get(25)?,
                    receipt_digest: row.get(26)?,
                    applied_at_ms: row.get(27)?,
                    expires_at_ms: row.get(28)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_PLAN_APPLICATION_REPLAY_READ")?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    if stored.plan_digest != plan_digest
        || stored.application_request_digest != application_request_digest
    {
        bail!("COMPUTE_PLUGIN_PLAN_ID_CONFLICT: plan ID is bound to another signed request");
    }
    let (signed_plan, admission, inventory_after) =
        validate_stored_application(transaction, plan_id, &stored)?;
    let receipt: ComputePluginPlanApplicationReceipt =
        serde_json::from_str(&stored.receipt_json).context("COMPUTE_PLUGIN_PLAN_RECEIPT_JSON")?;
    let (new_candidates, candidate_handles) = read_created_candidates(
        transaction,
        plan_id,
        &stored.plan_digest,
        stored.application_inventory_revision,
        stored.applied_at_ms,
    )?;
    let released_candidates = read_released_candidates(
        transaction,
        plan_id,
        &stored.plan_digest,
        stored.applied_at_ms,
    )?;
    let downloads = read_downloads(
        transaction,
        plan_id,
        &stored.plan_digest,
        stored.applied_at_ms,
    )?;
    validate_replayed_children(
        &signed_plan,
        &inventory_after,
        &new_candidates,
        &released_candidates,
        &downloads,
    )?;
    let actual_download_bytes = downloads
        .iter()
        .try_fold(0_i64, |sum, download| sum.checked_add(download.size_bytes));
    let prepared_released_claims = transaction
        .query_row(
            r#"SELECT COUNT(*)
            FROM fetch_claims AS claim
            JOIN candidate_owners AS candidate
              ON candidate.candidate_token = claim.candidate_token
            WHERE candidate.closed_by_plan_id = ?1
              AND candidate.state = 'released'
              AND claim.state = 'prepared'"#,
            [plan_id],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_RELEASED_CLAIMS_READ")?;
    let current_fences = transaction
        .query_row(
            r#"SELECT state_revision, inventory_revision, authority_epoch, process_owner_epoch
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_CURRENT_EPOCHS_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))?;
    let stale_prepared_claims = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM fetch_claims
            WHERE state = 'prepared'
              AND (authority_epoch <> ?1 OR process_owner_epoch <> ?2)"#,
            params![current_fences.2, current_fences.3],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_STALE_CLAIMS_READ")?;
    if receipt.schema != PLAN_APPLICATION_RECEIPT_SCHEMA
        || receipt.plan_id != plan_id
        || receipt.plan_digest != stored.plan_digest
        || receipt.application_request_digest != stored.application_request_digest
        || receipt.admission_bindings_digest != stored.admission_bindings_digest
        || receipt.inventory_before_revision != stored.expected_inventory_revision
        || receipt.inventory_before_digest != stored.expected_inventory_digest
        || receipt.inventory_after_revision != stored.application_inventory_revision
        || receipt.inventory_after_digest != stored.inventory_after_digest
        || receipt.application_state_revision != stored.application_state_revision
        || receipt.authority_epoch != stored.authority_epoch_at_apply
        || receipt.keyring_bundle_revision != stored.keyring_bundle_revision
        || receipt.publisher_keyring.revision != stored.publisher_keyring_revision
        || receipt.publisher_keyring.digest != stored.publisher_keyring_digest
        || receipt.control_keyring.revision != stored.control_keyring_revision
        || receipt.control_keyring.digest != stored.control_keyring_digest
        || receipt.control_signing_key_fingerprint != stored.control_signing_key_fingerprint
        || receipt.applied_at_ms != stored.applied_at_ms
        || receipt.new_candidates != new_candidates
        || receipt.released_candidates != released_candidates
        || receipt.downloads != downloads
        || receipt.download_bytes != stored.download_bytes
        || actual_download_bytes != Some(stored.download_bytes)
        || prepared_released_claims != 0
        || stale_prepared_claims != 0
        || current_fences.0 < stored.application_state_revision
        || current_fences.1 < stored.application_inventory_revision
        || current_fences.2 < stored.authority_epoch_at_apply
        || current_fences.3 < 0
        || i64::try_from(new_candidates.len()).ok() != Some(stored.new_candidate_count)
        || i64::try_from(released_candidates.len()).ok() != Some(stored.closed_candidate_count)
        || i64::try_from(downloads.len()).ok() != Some(stored.download_count)
    {
        bail!("COMPUTE_PLUGIN_PLAN_APPLICATION_REPLAY_CORRUPT");
    }
    let execution_plan = restore_execution_plan(signed_plan, admission)?;
    Ok(Some(ComputePluginPlanApplicationResult::new(
        ComputePluginPlanApplicationDisposition::Replayed,
        receipt,
        candidate_handles,
        execution_plan,
    )))
}

pub(super) fn persist_plan_application(
    transaction: &Transaction<'_>,
    authority: &AuthorityPlanApplicationState,
    keyring: &PersistedComputePluginKeyringSnapshot,
    request: &PreparedPlanApplicationRequest,
    admitted: &AdmittedComputePluginInstallPlan,
    projected: ProjectedPlanApplication,
    applied_at_ms: i64,
) -> Result<ComputePluginPlanApplicationResult> {
    let plan = admitted.plan();
    let inventory_after_json = serde_json::to_string(&projected.inventory_after)
        .context("COMPUTE_PLUGIN_PLAN_INVENTORY_AFTER_JSON")?;
    let inventory_after_digest = jcs_sha256_hex(&projected.inventory_after)?;
    let admission_bindings_json = serde_json::to_string(&projected.admission_bindings)
        .context("COMPUTE_PLUGIN_PLAN_ADMISSION_BINDINGS_JSON")?;
    let admission_bindings_digest = jcs_sha256_hex(&projected.admission_bindings)?;
    let signed_plan_json =
        serde_json::to_string(admitted.signed_plan()).context("COMPUTE_PLUGIN_PLAN_SIGNED_JSON")?;
    let signed_manifests_json = serde_json::to_string(&request.signed_manifests)
        .context("COMPUTE_PLUGIN_PLAN_MANIFESTS_JSON")?;
    let application_state_revision = authority
        .state_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_STATE_REVISION_OVERFLOW"))?;
    let authority_epoch = authority
        .authority_epoch
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_AUTHORITY_EPOCH_OVERFLOW"))?;
    let download_bytes = projected
        .downloads
        .iter()
        .try_fold(0_i64, |sum, download| sum.checked_add(download.size_bytes))
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_DOWNLOAD_BYTES_OVERFLOW"))?;
    let mut new_candidates = projected
        .candidates_to_create
        .iter()
        .map(|candidate| candidate.receipt())
        .collect::<Vec<_>>();
    new_candidates.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    let mut released_candidates = projected
        .candidates_to_release
        .iter()
        .map(|candidate| candidate.receipt())
        .collect::<Vec<_>>();
    released_candidates.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    let downloads = projected
        .downloads
        .iter()
        .map(|download| download.receipt())
        .collect::<Vec<_>>();
    let receipt = ComputePluginPlanApplicationReceipt {
        schema: PLAN_APPLICATION_RECEIPT_SCHEMA.to_string(),
        plan_id: plan.plan_id.clone(),
        plan_digest: admitted.plan_digest().to_string(),
        application_request_digest: request.application_request_digest.clone(),
        admission_bindings_digest: admission_bindings_digest.clone(),
        inventory_before_revision: authority.inventory.inventory_revision,
        inventory_before_digest: authority.inventory_digest.clone(),
        inventory_after_revision: projected.inventory_after.inventory_revision,
        inventory_after_digest: inventory_after_digest.clone(),
        application_state_revision,
        authority_epoch,
        keyring_bundle_revision: keyring.bundle_revision(),
        publisher_keyring: keyring.publisher_binding().clone(),
        control_keyring: keyring.control_binding().clone(),
        control_signing_key_fingerprint: admitted.control_signing_key_fingerprint().to_string(),
        new_candidates,
        released_candidates,
        downloads,
        download_bytes,
        applied_at_ms,
    };
    let receipt_json =
        serde_json::to_string(&receipt).context("COMPUTE_PLUGIN_PLAN_RECEIPT_SERIALIZE")?;
    let receipt_digest = jcs_sha256_hex(&receipt)?;
    let expires_at_ms = DateTime::parse_from_rfc3339(&plan.expires_at)
        .context("COMPUTE_PLUGIN_PLAN_EXPIRES_AT_PERSIST")?
        .timestamp_millis();
    insert_plan_application(
        transaction,
        authority,
        keyring,
        request,
        admitted,
        &projected,
        &signed_plan_json,
        &signed_manifests_json,
        &admission_bindings_json,
        &admission_bindings_digest,
        &inventory_after_json,
        &inventory_after_digest,
        application_state_revision,
        authority_epoch,
        download_bytes,
        &receipt_json,
        &receipt_digest,
        applied_at_ms,
        expires_at_ms,
    )?;
    release_candidates(
        transaction,
        &projected.candidates_to_release,
        &plan.plan_id,
        admitted.plan_digest(),
        applied_at_ms,
    )?;
    revoke_for_plan_authority_epoch_advance(
        transaction,
        authority.authority_epoch,
        authority_epoch,
        applied_at_ms,
    )?;
    insert_candidates_and_downloads(
        transaction,
        &projected,
        &plan.plan_id,
        admitted.plan_digest(),
        projected.inventory_after.inventory_revision,
        applied_at_ms,
    )?;
    insert_event_and_seal(transaction, &receipt, &receipt_digest, applied_at_ms)?;
    update_authority_meta(
        transaction,
        authority,
        &projected.inventory_after,
        &inventory_after_json,
        &inventory_after_digest,
        application_state_revision,
        authority_epoch,
        applied_at_ms,
    )?;
    let replayed = replay_plan_application(
        transaction,
        &plan.plan_id,
        admitted.plan_digest(),
        &request.application_request_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_APPLICATION_POST_WRITE_MISSING"))?;
    if replayed.receipt() != &receipt {
        bail!("COMPUTE_PLUGIN_PLAN_APPLICATION_POST_WRITE_MISMATCH");
    }
    Ok(ComputePluginPlanApplicationResult::new(
        ComputePluginPlanApplicationDisposition::Applied,
        receipt,
        replayed.candidate_handles().to_vec(),
        replayed.execution_plan().clone(),
    ))
}

struct StoredPlanApplication {
    plan_digest: String,
    application_request_digest: String,
    signed_plan_envelope_digest: String,
    signed_manifest_set_digest: String,
    signed_plan_json: String,
    signed_manifests_json: String,
    admission_bindings_json: String,
    admission_bindings_digest: String,
    inventory_after_json: String,
    inventory_after_digest: String,
    expected_inventory_revision: i64,
    expected_inventory_digest: String,
    application_inventory_revision: i64,
    application_state_revision: i64,
    authority_epoch_at_apply: i64,
    keyring_bundle_revision: i64,
    publisher_keyring_revision: i64,
    publisher_keyring_digest: String,
    control_keyring_revision: i64,
    control_keyring_digest: String,
    control_signing_key_fingerprint: String,
    new_candidate_count: i64,
    closed_candidate_count: i64,
    download_count: i64,
    download_bytes: i64,
    receipt_json: String,
    receipt_digest: String,
    applied_at_ms: i64,
    expires_at_ms: i64,
}

fn validate_stored_application(
    transaction: &Transaction<'_>,
    plan_id: &str,
    stored: &StoredPlanApplication,
) -> Result<(
    SignedComputePluginInstallPlan,
    PersistedAdmissionBindings,
    ComputePluginInventorySnapshot,
)> {
    let signed_plan: SignedComputePluginInstallPlan =
        serde_json::from_str(&stored.signed_plan_json)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_SIGNED_PLAN_JSON")?;
    let manifests: Vec<SignedComputePluginManifest> =
        serde_json::from_str(&stored.signed_manifests_json)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_MANIFESTS_JSON")?;
    let request = prepare_application_request(&signed_plan, &manifests)?;
    let admission: PersistedAdmissionBindings =
        serde_json::from_str(&stored.admission_bindings_json)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_ADMISSION_JSON")?;
    let inventory: ComputePluginInventorySnapshot =
        serde_json::from_str(&stored.inventory_after_json)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_INVENTORY_JSON")?;
    let receipt: ComputePluginPlanApplicationReceipt =
        serde_json::from_str(&stored.receipt_json)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_RECEIPT_JSON")?;
    let expected_application_inventory_revision = signed_plan
        .plan
        .expected_inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_INVENTORY_OVERFLOW"))?;
    let expires_at_ms = DateTime::parse_from_rfc3339(&signed_plan.plan.expires_at)
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_EXPIRES_AT")?
        .timestamp_millis();
    let expected_observed_at = DateTime::<Utc>::from_timestamp_millis(stored.applied_at_ms)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_APPLIED_AT"))?
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    validate_admission_bindings(transaction, &signed_plan, &manifests, &admission, stored)?;
    let seal = transaction
        .query_row(
            r#"SELECT application_request_digest, receipt_digest, sealed_at_ms
        FROM plan_application_seals
        WHERE plan_id = ?1 AND plan_digest = ?2"#,
            params![plan_id, &stored.plan_digest],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_SEAL_READ")?;
    let event = transaction
        .query_row(
            r#"SELECT event_digest, payload_json, recorded_at_ms FROM plan_events
        WHERE plan_id = ?1 AND plan_digest = ?2 AND event_index = 0 AND event_type = 'applied'"#,
            params![plan_id, &stored.plan_digest],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_EVENT_READ")?;
    let event_valid = if let Some((event_digest, payload_json, recorded_at_ms)) = event {
        let payload: StoredPlanAppliedEvent =
            serde_json::from_str(&payload_json).context("COMPUTE_PLUGIN_PLAN_REPLAY_EVENT_JSON")?;
        jcs_sha256_hex(&payload)? == event_digest
            && payload.schema == "elon.compute_plugin.plan_applied_event.v1"
            && payload.plan_id == plan_id
            && payload.plan_digest == stored.plan_digest
            && payload.application_request_digest == stored.application_request_digest
            && payload.admission_bindings_digest == stored.admission_bindings_digest
            && payload.inventory_after_revision == stored.application_inventory_revision
            && payload.inventory_after_digest == stored.inventory_after_digest
            && payload.receipt_digest == stored.receipt_digest
            && payload.recorded_at_ms == stored.applied_at_ms
            && recorded_at_ms == stored.applied_at_ms
    } else {
        false
    };
    if signed_plan.plan.plan_id != plan_id
        || signed_plan.plan_digest != stored.plan_digest
        || signed_plan.plan.expected_inventory_revision != stored.expected_inventory_revision
        || signed_plan.plan.expected_inventory_digest != stored.expected_inventory_digest
        || signed_plan.plan.publisher_keyring.revision != stored.publisher_keyring_revision
        || signed_plan.plan.publisher_keyring.digest != stored.publisher_keyring_digest
        || signed_plan.plan.control_keyring.revision != stored.control_keyring_revision
        || signed_plan.plan.control_keyring.digest != stored.control_keyring_digest
        || expected_application_inventory_revision != stored.application_inventory_revision
        || expires_at_ms != stored.expires_at_ms
        || request.application_request_digest != stored.application_request_digest
        || request.signed_plan_envelope_digest != stored.signed_plan_envelope_digest
        || request.signed_manifest_set_digest != stored.signed_manifest_set_digest
        || request.signed_manifests != manifests
        || jcs_sha256_hex(&admission)? != stored.admission_bindings_digest
        || inventory.schema != COMPUTE_PLUGIN_INVENTORY_SCHEMA
        || inventory.inventory_revision != stored.application_inventory_revision
        || inventory.desired_policy_revision != signed_plan.plan.desired_policy_revision
        || inventory.sharing_enabled != signed_plan.plan.sharing_enabled
        || inventory.observed_at != expected_observed_at
        || inventory.plugins.len() > MAX_COMPUTE_PLUGIN_INVENTORY_RECORDS
        || inventory
            .plugins
            .windows(2)
            .any(|pair| pair[0].plugin_id >= pair[1].plugin_id)
        || inventory
            .plugins
            .iter()
            .any(|record| !local_record_shape_is_valid(record))
        || jcs_sha256_hex(&inventory)? != stored.inventory_after_digest
        || jcs_sha256_hex(&receipt)? != stored.receipt_digest
        || seal
            != Some((
                stored.application_request_digest.clone(),
                stored.receipt_digest.clone(),
                stored.applied_at_ms,
            ))
        || !event_valid
    {
        bail!("COMPUTE_PLUGIN_PLAN_APPLICATION_STORAGE_CORRUPT");
    }
    Ok((signed_plan, admission, inventory))
}

fn validate_admission_bindings(
    transaction: &Transaction<'_>,
    signed_plan: &SignedComputePluginInstallPlan,
    manifests: &[SignedComputePluginManifest],
    admission: &PersistedAdmissionBindings,
    stored: &StoredPlanApplication,
) -> Result<()> {
    let expected_manifest_count = signed_plan
        .plan
        .items
        .iter()
        .filter(|item| manifest_release_for_item(item).is_some())
        .count();
    if admission.schema != ADMISSION_BINDINGS_SCHEMA
        || admission.admitted_at_ms != stored.applied_at_ms
        || admission.control_signing_key_fingerprint != stored.control_signing_key_fingerprint
        || !is_sha256(&admission.control_signing_key_fingerprint)
        || admission.manifests.len() != expected_manifest_count
        || manifests.len() != expected_manifest_count
        || admission
            .manifests
            .windows(2)
            .any(|pair| pair[0].item_index >= pair[1].item_index)
    {
        bail!("COMPUTE_PLUGIN_PLAN_REPLAY_ADMISSION_BINDINGS");
    }
    validate_sealed_key_binding(
        transaction,
        stored,
        "control_install_plan",
        "",
        &signed_plan.signature.signing_key_id,
        &admission.control_signing_key_fingerprint,
    )?;
    for binding in &admission.manifests {
        let item_index = usize::try_from(binding.item_index)
            .context("COMPUTE_PLUGIN_PLAN_REPLAY_ADMISSION_ITEM_INDEX")?;
        let item = signed_plan
            .plan
            .items
            .get(item_index)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_ADMISSION_ITEM"))?;
        let target = manifest_release_for_item(item)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_ADMISSION_TARGET"))?;
        let matching = manifests
            .iter()
            .filter(|manifest| manifest_release_ref(manifest) == binding.release)
            .collect::<Vec<_>>();
        if !manifest_binding_action_is_valid(item)
            || target != &binding.release
            || matching.len() != 1
            || matching[0].manifest.publisher_id != binding.publisher_id
            || matching[0].signature.signing_key_id != binding.signing_key_id
            || !is_identifier(&binding.publisher_id)
            || !is_identifier(&binding.signing_key_id)
            || !is_sha256(&binding.signing_key_fingerprint)
            || binding.signing_key_fingerprint == admission.control_signing_key_fingerprint
        {
            bail!("COMPUTE_PLUGIN_PLAN_REPLAY_ADMISSION_MANIFEST_BINDING");
        }
        validate_sealed_key_binding(
            transaction,
            stored,
            "publisher_manifest",
            &binding.publisher_id,
            &binding.signing_key_id,
            &binding.signing_key_fingerprint,
        )?;
    }
    Ok(())
}

fn validate_sealed_key_binding(
    transaction: &Transaction<'_>,
    stored: &StoredPlanApplication,
    purpose: &str,
    subject_id: &str,
    signing_key_id: &str,
    expected_fingerprint: &str,
) -> Result<()> {
    let key = transaction
        .query_row(
            r#"SELECT fingerprint_sha256, status, not_before_ms, not_after_ms, revoked_at_ms
            FROM keyring_keys
            WHERE bundle_revision = ?1 AND purpose = ?2 AND subject_id = ?3
              AND signing_key_id = ?4"#,
            params![
                stored.keyring_bundle_revision,
                purpose,
                subject_id,
                signing_key_id,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_PLAN_REPLAY_KEY_BINDING_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLAN_REPLAY_KEY_BINDING_MISSING"))?;
    if key.0 != expected_fingerprint
        || key.1 != "active"
        || stored.applied_at_ms < key.2
        || stored.applied_at_ms >= key.3
        || key.4.is_some()
        || !is_identifier(signing_key_id)
        || !is_sha256(expected_fingerprint)
    {
        bail!("COMPUTE_PLUGIN_PLAN_REPLAY_KEY_BINDING_CHANGED");
    }
    Ok(())
}

fn manifest_release_ref(manifest: &SignedComputePluginManifest) -> ComputePluginReleaseRef {
    ComputePluginReleaseRef {
        plugin_id: manifest.manifest.plugin_id.clone(),
        plugin_version: manifest.manifest.plugin_version.clone(),
        target_id: manifest.manifest.target.target_id.clone(),
        manifest_digest: manifest.manifest_digest.clone(),
        package_digest: manifest.manifest.package.package_digest.clone(),
    }
}
