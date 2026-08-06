use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction};

use super::{
    projection::project_candidate_health_quarantine,
    ComputePluginCandidateHealthQuarantineAuthorityFacts,
    ComputePluginCandidateHealthQuarantineAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::{
        validate_hashed_candidate_health_failure_observation,
        ValidatedCandidateHealthFailurePublication,
    },
    lifecycle::SLOT_STAGED,
    local_authority::plan_application::read_authority_plan_application_state,
};

pub(super) fn read_candidate_health_quarantine_binding(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateHealthQuarantineAuthoritySession<'_>,
    publication: &ValidatedCandidateHealthFailurePublication<'_>,
) -> Result<ComputePluginCandidateHealthQuarantineAuthorityFacts> {
    validate_hashed_candidate_health_failure_observation(publication.observation())?;
    let observation = &publication.observation().observation;
    let staged = publication.staged();
    let staging_key = staged.recovery_key();
    let staging_slot = staging_key.slot_expectation();
    let failed_at_ms = parse_utc_ms(&observation.failed_at)?;
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let guard = staged.archive().snapshot_cancellation_guard();
    session.validate_source(&guard)?;

    if session.trusted_now_ms() != failed_at_ms
        || observation.installation_id_digest != session.installation_id_digest()
        || observation.clock_epoch_digest != session.clock_epoch_digest()
        || observation.process_owner_epoch != session.process_owner_epoch()
        || observation.authority_state_revision != authority.state_revision
        || observation.inventory_revision != authority.inventory.inventory_revision
        || observation.inventory_digest != authority.inventory_digest
        || observation.authority_epoch != authority.authority_epoch
        || observation.process_owner_epoch != authority.process_owner_epoch
        || authority.trusted_time_high_water_ms >= failed_at_ms
        || !authority.sharing_enabled
        || observation.candidate_token_digest != staging_key.candidate_token_digest()
        || observation.staging_id != staging_key.staging_id()
        || observation.staging_receipt_digest != staged.receipt().receipt_digest()
        || observation.staging_run_digest != staging_key.staging_run_digest()
        || observation.release != staging_slot.release
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_AUTHORITY_BINDING_CHANGED");
    }
    validate_staged_inventory(
        &authority.inventory,
        &staging_slot.plugin_id,
        &staging_slot.slot_ref,
        &staging_slot.release,
    )?;
    validate_staging_receipt_row(transaction, publication)?;
    let projection = project_candidate_health_quarantine(
        &authority,
        &staging_slot.plugin_id,
        &staging_slot.slot_ref,
        &staging_slot.release,
        &session.trusted_now,
    )?;
    session.validate_source(&guard)?;

    Ok(ComputePluginCandidateHealthQuarantineAuthorityFacts {
        authority_state_revision_before: authority.state_revision,
        authority_state_revision_after: projection.state_revision,
        inventory_revision_before: authority.inventory.inventory_revision,
        inventory_revision_after: projection.inventory.inventory_revision,
        inventory_digest_before: authority.inventory_digest,
        inventory_digest_after: projection.inventory_digest,
        authority_epoch_before: authority.authority_epoch,
        authority_epoch_after: projection.authority_epoch,
        process_owner_epoch: authority.process_owner_epoch,
        trusted_time_high_water_ms_before: authority.trusted_time_high_water_ms,
        failed_at_ms,
        candidate_token_digest: staging_key.candidate_token_digest().to_string(),
        staging_id: staging_key.staging_id().to_string(),
        staging_receipt_digest: staged.receipt().receipt_digest().to_string(),
        staging_run_digest: staging_key.staging_run_digest().to_string(),
        plugin_id: staging_slot.plugin_id.clone(),
        slot_ref: staging_slot.slot_ref.clone(),
        release: staging_slot.release.clone(),
    })
}

pub(super) fn validate_staged_inventory(
    inventory: &crate::node_agent_compute_plugin_host::lifecycle::ComputePluginInventorySnapshot,
    plugin_id: &str,
    slot_ref: &str,
    release: &crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
) -> Result<()> {
    let plugin = inventory
        .plugins
        .iter()
        .find(|plugin| plugin.plugin_id == plugin_id)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_PLUGIN_MISSING"))?;
    let slot = plugin
        .slots
        .iter()
        .find(|slot| slot.slot_ref == slot_ref)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_SLOT_MISSING"))?;
    if plugin.candidate_slot_ref.as_deref() != Some(slot_ref)
        || slot.phase != SLOT_STAGED
        || &slot.release != release
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_SLOT_CHANGED");
    }
    Ok(())
}

fn validate_staging_receipt_row(
    transaction: &Transaction<'_>,
    publication: &ValidatedCandidateHealthFailurePublication<'_>,
) -> Result<()> {
    let staged = publication.staged();
    let key = staged.recovery_key();
    let receipt = staged.receipt().receipt();
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_staging_receipts
            WHERE staging_id = ?1 AND candidate_token = ?2
              AND candidate_token_digest = ?3 AND staging_run_digest = ?4
              AND receipt_digest = ?5 AND authority_state_revision_after = ?6
              AND inventory_revision_after = ?7 AND inventory_digest_after = ?8
              AND authority_epoch_after = ?9 AND process_owner_epoch = ?10"#,
            params![
                key.staging_id(),
                key.candidate_token(),
                key.candidate_token_digest(),
                key.staging_run_digest(),
                staged.receipt().receipt_digest(),
                receipt.authority_state_revision_after(),
                receipt.inventory_revision_after(),
                receipt.inventory_digest_after(),
                receipt.authority_epoch_after(),
                key.process_owner_epoch(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_STAGING_RECEIPT_READ")?;
    let owner_count = transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_owners WHERE candidate_token = ?1 AND state = 'owned'",
            params![key.candidate_token()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_OWNER_READ")?;
    let live_health_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_health_receipts
               WHERE candidate_token = ?1 AND expires_at_ms > ?2"#,
            params![
                key.candidate_token(),
                parse_utc_ms(&publication.observation().observation.failed_at)?
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_HEALTH_READ")?;
    if count != 1 || owner_count != 1 || live_health_count != 0 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_STAGING_RECEIPT_CHANGED");
    }
    Ok(())
}

fn parse_utc_ms(value: &str) -> Result<i64> {
    Ok(DateTime::parse_from_rfc3339(value)
        .context("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_FAILED_AT_INVALID")?
        .with_timezone(&Utc)
        .timestamp_millis())
}
