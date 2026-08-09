use std::time::Instant;

use anyhow::{bail, Context, Result};
use rusqlite::Transaction;

use super::{
    head::{read_head, WorkAdmissionHead},
    installed::validate_installed_projection,
    source::{read_current_source, CurrentWorkAdmissionSource},
    ComputePluginInstalledWorkAdmissionAuthorityFacts,
    ComputePluginPostRevalidationWorkAdmissionAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    lifecycle::{
        ACTIVATION_ENABLED, ADMISSION_ALLOWED, DESIRED_PRESENCE_PRESENT, RUNTIME_STOPPED,
        SLOT_INSTALLED,
    },
    local_authority::plan_application::{
        read_authority_plan_application_state, AuthorityPlanApplicationState,
    },
    work_admission_contract::RevalidatedInstalledWorkAdmission,
};

const I_JSON_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(super) fn read_installed_work_admission_binding(
    transaction: &Transaction<'_>,
    session: &ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'_>,
    revalidated: &RevalidatedInstalledWorkAdmission<'_>,
) -> Result<ComputePluginInstalledWorkAdmissionAuthorityFacts> {
    session.process_fence.ensure_process_owner_current()?;
    revalidated.trusted_time().ensure_live(Instant::now())?;
    if !session.was_observed_strictly_after(revalidated.revalidated_at())
        || !session.plan_application_matches_observation(revalidated.trusted_time())
        || session.trusted_now_ms() < revalidated.trusted_time().trusted_now().timestamp_millis()
        || revalidated.trusted_time().installation_id_digest() != session.installation_id_digest()
        || revalidated.trusted_time().clock_epoch_digest() != session.clock_epoch_digest()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_REVALIDATION_FENCE_CHANGED");
    }
    let installed = revalidated.installed();
    validate_installed_projection(transaction, installed, session.installation_id_digest())?;
    let install = installed.receipts().install();
    let promotion = installed.receipts().promotion();
    let install_body = install.receipt();
    let promotion_body = promotion.receipt();

    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let authority_updated_at_ms = transaction
        .query_row(
            "SELECT updated_at_ms FROM authority_meta WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_TIME_READ")?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_fence.process_owner_epoch()
        || authority.inventory.inventory_revision != session.plan_application_inventory_revision
        || authority.inventory_digest.as_str() != session.plan_application_inventory_digest.as_str()
        || authority_updated_at_ms != authority.trusted_time_high_water_ms
        || session.trusted_now_ms() <= authority.trusted_time_high_water_ms
        || !authority.sharing_enabled
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_FENCE_CHANGED");
    }

    let mut records = authority
        .inventory
        .plugins
        .iter()
        .filter(|record| record.plugin_id == install_body.plugin_id());
    let record = records
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RECORD_MISSING"))?;
    if records.next().is_some() {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECORD_DUPLICATED");
    }
    let active_slot_ref = record
        .active_slot_ref
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_ACTIVE_SLOT_MISSING"))?;
    let mut slots = record.slots.iter().filter(|slot| {
        slot.slot_ref == active_slot_ref
            && slot.phase == SLOT_INSTALLED
            && slot.release == *install_body.release()
    });
    let slot = slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_SLOT_MISSING"))?;
    if slots.next().is_some()
        || active_slot_ref != install_body.slot_ref()
        || install_body.plugin_id() != promotion_body.plugin_id()
        || install_body.slot_ref() != promotion_body.slot_ref()
        || install_body.release() != promotion_body.release()
        || record.install_generation != install_body.install_generation_after()
        || record.activation_generation != promotion_body.activation_generation_after()
        || record.desired_presence != DESIRED_PRESENCE_PRESENT
        || record.desired_activation != ACTIVATION_ENABLED
        || record.admission != ADMISSION_ALLOWED
        || record.candidate_slot_ref.is_some()
        || record.runtime.phase != RUNTIME_STOPPED
        || record.runtime.slot_ref.is_some()
        || record.runtime.runner_digest.is_some()
        || record.health.is_some()
        || record.active_attempts != 0
        || slot.installed_at.is_none()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_SOURCE_NOT_QUIESCENT");
    }

    let source = read_current_source(
        transaction,
        &authority,
        authority_updated_at_ms,
        record,
        install_body.release(),
        session.trusted_now_ms(),
    )?;
    if record.permission_grant_digest.as_deref() != Some(source.grant.grant_digest.as_str())
        || install_body.signed_manifest_envelope_digest() != source.signed_manifest_envelope_digest
        || promotion_body.signed_manifest_envelope_digest()
            != source.signed_manifest_envelope_digest
        || source.plan_id.as_str() != session.plan_application_plan_id.as_str()
        || source.plan_digest.as_str() != session.plan_application_plan_digest.as_str()
        || source.application_receipt_digest.as_str()
            != session.plan_application_receipt_digest.as_str()
        || source.application_inventory_revision != session.plan_application_inventory_revision
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_SOURCE_BINDING_CHANGED");
    }
    validate_no_prepared_work(transaction)?;

    let head = read_head(transaction, install_body.plugin_id())?;
    if head.as_ref().is_some_and(|head| {
        head.installation_id_digest != session.installation_id_digest()
            || head.plugin_id != install_body.plugin_id()
            || head.generation <= 0
            || head.updated_at_ms >= session.trusted_now_ms()
    }) {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_PREVIOUS_HEAD_CHANGED");
    }
    let work_admission_generation_before = head.as_ref().map_or(0, |head| head.generation);
    validate_numeric_projection(
        &authority,
        authority_updated_at_ms,
        record,
        &source,
        head.as_ref(),
        session.trusted_now_ms(),
    )?;
    let work_admission_generation_after = work_admission_generation_before
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_GENERATION_EXHAUSTED"))?;
    let authority_state_revision_after = authority
        .state_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_STATE_EXHAUSTED"))?;
    let authority_epoch_after = authority
        .authority_epoch
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_EPOCH_EXHAUSTED"))?;
    let sharing = authority
        .sharing_authorization
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_SHARING_MISSING"))?;

    session.process_fence.ensure_process_owner_current()?;
    revalidated.trusted_time().ensure_live(Instant::now())?;
    Ok(ComputePluginInstalledWorkAdmissionAuthorityFacts {
        signed_manifest: source.signed_manifest,
        grant: source.grant,
        selected_host_api_revision: authority.host_api_revision,
        action: source.action,
        plugin_id: install_body.plugin_id().to_string(),
        slot_ref: install_body.slot_ref().to_string(),
        release: install_body.release().clone(),
        install_receipt_id: install_body.install_receipt_id().to_string(),
        install_receipt_digest: install.receipt_digest().to_string(),
        promotion_receipt_id: promotion_body.promotion_receipt_id().to_string(),
        promotion_receipt_digest: promotion.receipt_digest().to_string(),
        signed_manifest_envelope_digest: source.signed_manifest_envelope_digest,
        plan_id: source.plan_id,
        plan_digest: source.plan_digest,
        signed_plan_envelope_digest: source.signed_plan_envelope_digest,
        signed_manifest_set_digest: source.signed_manifest_set_digest,
        application_request_digest: source.application_request_digest,
        application_receipt_digest: source.application_receipt_digest,
        admission_bindings_digest: source.admission_bindings_digest,
        application_inventory_revision: source.application_inventory_revision,
        policy_revision: authority.desired_policy_revision,
        sharing_authorization_ref: sharing.authorization_ref.clone(),
        sharing_authorization_revision: sharing.revision,
        sharing_authorization_digest: sharing.digest.clone(),
        policy_binding_receipt_digest: source.policy_binding_receipt_digest,
        policy_revocation_receipt_digest: source.policy_revocation_receipt_digest,
        node_profile_digest: authority.node_profile_digest,
        manifest_catalog_revision: authority.manifest_catalog_revision,
        manifest_catalog_digest: source.manifest_catalog_digest,
        manifest_catalog_binding_receipt_digest: source.manifest_catalog_binding_receipt_digest,
        keyring_bundle_revision: authority.keyring_bundle_revision,
        publisher_keyring_revision: authority.publisher_keyring.revision,
        publisher_keyring_digest: authority.publisher_keyring.digest,
        control_keyring_revision: authority.control_keyring.revision,
        control_keyring_digest: authority.control_keyring.digest,
        install_generation: record.install_generation,
        activation_generation: record.activation_generation,
        runtime_generation: record.runtime.runtime_generation,
        work_admission_generation_before,
        work_admission_generation_after,
        previous_work_admission_id: head.as_ref().map(|head| head.work_admission_id.clone()),
        previous_work_admission_receipt_digest: head
            .as_ref()
            .map(|head| head.receipt_digest.clone()),
        desired_presence: record.desired_presence.clone(),
        desired_activation: record.desired_activation.clone(),
        slot_phase: slot.phase.clone(),
        admission: record.admission.clone(),
        runtime_phase: record.runtime.phase.clone(),
        candidate_slot_present: record.candidate_slot_ref.is_some(),
        runtime_slot_present: record.runtime.slot_ref.is_some(),
        runtime_runner_digest_present: record.runtime.runner_digest.is_some(),
        health_present: record.health.is_some(),
        active_attempts: record.active_attempts,
        authority_state_revision_before: authority.state_revision,
        authority_state_revision_after,
        inventory_revision_before: authority.inventory.inventory_revision,
        inventory_revision_after: authority.inventory.inventory_revision,
        inventory_digest_before: authority.inventory_digest.clone(),
        inventory_digest_after: authority.inventory_digest,
        authority_epoch_before: authority.authority_epoch,
        authority_epoch_after,
        process_owner_epoch: authority.process_owner_epoch,
        trusted_time_high_water_ms_before: authority.trusted_time_high_water_ms,
        authority_updated_at_ms_before: authority_updated_at_ms,
        admitted_at_ms: session.trusted_now_ms(),
    })
}

fn validate_numeric_projection(
    authority: &AuthorityPlanApplicationState,
    authority_updated_at_ms: i64,
    record: &crate::node_agent_compute_plugin_host::lifecycle::ComputePluginLocalRecord,
    source: &CurrentWorkAdmissionSource,
    head: Option<&WorkAdmissionHead>,
    admitted_at_ms: i64,
) -> Result<()> {
    let sharing = authority
        .sharing_authorization
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_SHARING_MISSING"))?;
    let manifest = &source.signed_manifest.manifest;
    let runner = manifest
        .package
        .files
        .iter()
        .find(|file| file.relative_path == manifest.entrypoint.relative_path)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RUNNER_FILE_MISSING"))?;
    let resources = &source.grant.granted_resources;
    let work_generation = head.map_or(0, |value| value.generation);
    let positive_safe = [
        source.application_inventory_revision,
        authority.desired_policy_revision,
        sharing.revision,
        authority.manifest_catalog_revision,
        authority.keyring_bundle_revision,
        authority.publisher_keyring.revision,
        authority.control_keyring.revision,
        runner.size_bytes,
        resources.max_cpu_millicores,
        resources.max_memory_bytes,
        resources.max_disk_bytes,
        resources.max_processes,
        resources.max_sidecar_uptime_seconds,
        record.install_generation,
        record.activation_generation,
        authority.inventory.inventory_revision,
        authority.process_owner_epoch,
    ];
    if positive_safe
        .iter()
        .any(|value| *value <= 0 || *value > I_JSON_MAX_SAFE_INTEGER)
        || resources.max_vram_bytes < 0
        || resources.max_vram_bytes > I_JSON_MAX_SAFE_INTEGER
        || record.runtime.runtime_generation < 0
        || record.runtime.runtime_generation > I_JSON_MAX_SAFE_INTEGER
        || work_generation < 0
        || work_generation >= I_JSON_MAX_SAFE_INTEGER
        || authority.state_revision <= 0
        || authority.state_revision >= I_JSON_MAX_SAFE_INTEGER
        || authority.authority_epoch <= 0
        || authority.authority_epoch >= I_JSON_MAX_SAFE_INTEGER
        || authority.trusted_time_high_water_ms <= 0
        || authority.trusted_time_high_water_ms >= I_JSON_MAX_SAFE_INTEGER
        || authority_updated_at_ms <= 0
        || authority_updated_at_ms >= I_JSON_MAX_SAFE_INTEGER
        || admitted_at_ms <= authority.trusted_time_high_water_ms
        || admitted_at_ms <= authority_updated_at_ms
        || admitted_at_ms > I_JSON_MAX_SAFE_INTEGER
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_NUMERIC_PROJECTION_INVALID");
    }
    Ok(())
}

pub(super) fn validate_no_prepared_work(transaction: &Transaction<'_>) -> Result<()> {
    let count = transaction
        .query_row(
            r#"SELECT
              (SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared')
            + (SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared')"#,
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_PREPARED_WORK_READ")?;
    if count != 0 {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_PREPARED_WORK_PRESENT");
    }
    Ok(())
}
