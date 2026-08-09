use std::time::Instant;

use anyhow::{bail, Context, Result};
use rusqlite::Transaction;

use super::{
    head::{advance_head, read_head},
    insert::insert_receipt,
    readback::read_pair_required,
    validation::validate_permit,
    ComputePluginPostRevalidationWorkAdmissionAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    lifecycle::{RUNTIME_STOPPED, SLOT_INSTALLED},
    local_authority::plan_application::read_authority_plan_application_state_at_or_before_observation,
    work_admission_contract::ValidatedInstalledWorkAdmissionStorePermit,
};

pub(super) fn persist_installed_work_admission(
    transaction: &Transaction<'_>,
    session: &ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'_>,
    permit: ValidatedInstalledWorkAdmissionStorePermit<'_, '_>,
) -> Result<()> {
    session.ensure_trusted_time_live()?;
    permit
        .revalidated()
        .trusted_time()
        .ensure_live(Instant::now())?;
    session.process_fence.ensure_process_owner_current()?;
    validate_permit(session, &permit)?;

    let expected = permit.receipts();
    let receipt = expected.receipt().receipt();
    let generations = receipt.generations();
    advance_head(
        transaction,
        receipt.installation_id_digest(),
        receipt.plugin_id(),
        generations.work_admission_generation_before(),
        generations.work_admission_generation_after(),
        receipt.work_admission_id(),
        expected.receipt().receipt_digest(),
        generations.previous_work_admission_id(),
        generations.previous_work_admission_receipt_digest(),
        receipt.admitted_at_ms(),
    )?;
    insert_receipt(transaction, expected)?;

    let stored = read_pair_required(
        transaction,
        receipt.work_admission_id(),
        expected.receipt().receipt_digest(),
    )?;
    if &stored != expected {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_STORE_READBACK_CHANGED");
    }
    validate_postcondition(transaction, session, &permit)?;
    session.ensure_trusted_time_live()?;
    permit
        .revalidated()
        .trusted_time()
        .ensure_live(Instant::now())?;
    session.process_fence.ensure_process_owner_current()?;
    Ok(())
}

fn validate_postcondition(
    transaction: &Transaction<'_>,
    session: &ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'_>,
    permit: &ValidatedInstalledWorkAdmissionStorePermit<'_, '_>,
) -> Result<()> {
    let facts = permit.facts();
    let pair = permit.receipts();
    let receipt = pair.receipt().receipt();
    let generations = receipt.generations();
    let stored_head = read_head(transaction, receipt.plugin_id())?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_HEAD_READBACK_MISSING"))?;
    if stored_head.installation_id_digest != receipt.installation_id_digest()
        || stored_head.plugin_id != receipt.plugin_id()
        || stored_head.generation != generations.work_admission_generation_after()
        || stored_head.work_admission_id != receipt.work_admission_id()
        || stored_head.receipt_digest != pair.receipt().receipt_digest()
        || stored_head.previous_id.as_deref() != generations.previous_work_admission_id()
        || stored_head.previous_receipt_digest.as_deref()
            != generations.previous_work_admission_receipt_digest()
        || stored_head.updated_at_ms != receipt.admitted_at_ms()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_HEAD_READBACK_CHANGED");
    }

    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    let updated_at_ms = transaction
        .query_row(
            "SELECT updated_at_ms FROM authority_meta WHERE singleton = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_READBACK_TIME")?;
    let record = authority
        .inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == receipt.plugin_id());
    let slot = record.and_then(|record| {
        record.slots.iter().find(|slot| {
            slot.slot_ref == receipt.slot_ref()
                && slot.release == *receipt.release()
                && slot.phase == SLOT_INSTALLED
        })
    });
    if authority.installation_id_digest != receipt.installation_id_digest()
        || authority.state_revision != facts.authority_state_revision_after()
        || authority.inventory.inventory_revision != facts.inventory_revision_after()
        || authority.inventory_digest != facts.inventory_digest_after()
        || authority.authority_epoch != facts.authority_epoch_after()
        || authority.process_owner_epoch != facts.process_owner_epoch()
        || authority.trusted_time_high_water_ms != receipt.admitted_at_ms()
        || updated_at_ms != receipt.admitted_at_ms()
        || record.is_none_or(|record| {
            record.install_generation != facts.install_generation()
                || record.activation_generation != facts.activation_generation()
                || record.active_slot_ref.as_deref() != Some(receipt.slot_ref())
                || record.candidate_slot_ref.is_some()
                || record.runtime.phase != RUNTIME_STOPPED
                || record.runtime.runtime_generation != facts.runtime_generation()
                || record.runtime.slot_ref.is_some()
                || record.runtime.runner_digest.is_some()
                || record.health.is_some()
                || record.active_attempts != 0
                || record.permission_grant_digest.as_deref()
                    != Some(permit.facts().grant().grant_digest.as_str())
        })
        || slot.is_none()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_READBACK_CHANGED");
    }
    Ok(())
}
