use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    projection::project_candidate_promotion, ComputePluginCandidatePromotionAuthorityFacts,
    ComputePluginPostRevalidationPromotionAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::validate_hashed_candidate_health_observation,
    candidate_promotion_contract::{
        ComputePluginPreviousActiveSlot, RevalidatedCandidatePromotion,
    },
    identity::ComputePluginReleaseRef,
    lifecycle::{RUNTIME_STOPPED, SLOT_INSTALLED, SLOT_STAGED},
    local_authority::plan_application::read_authority_plan_application_state,
    manifest_validation::is_sha256,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};

struct CandidateOwnerRow {
    plugin_id: String,
    slot_ref: String,
    candidate_generation: i64,
    release: ComputePluginReleaseRef,
    release_json: String,
    permission_grant_digest: String,
    owner_plan_id: String,
    owner_plan_digest: String,
    application_inventory_revision: i64,
}

pub(super) fn read_candidate_promotion_binding(
    transaction: &Transaction<'_>,
    session: &ComputePluginPostRevalidationPromotionAuthoritySession<'_>,
    promotion: &RevalidatedCandidatePromotion<'_>,
) -> Result<ComputePluginCandidatePromotionAuthorityFacts> {
    let guard = promotion.staged().archive().snapshot_cancellation_guard();
    session.validate_source(&guard)?;
    validate_hashed_candidate_health_observation(promotion.health().observation())?;
    if !session.was_observed_strictly_after(promotion.revalidated_at())
        || promotion.trusted_time().trusted_now().timestamp_millis() != session.trusted_now_ms()
        || promotion.trusted_time().installation_id_digest() != session.installation_id_digest()
        || promotion.trusted_time().clock_epoch_digest() != session.clock_epoch_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_REVALIDATION_FENCE_CHANGED");
    }

    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let staged = promotion.staged();
    let staging_key = staged.recovery_key();
    let slot = staging_key.slot_expectation();
    let health = promotion.health();
    let health_receipt = health.receipt();
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || !authority.sharing_enabled
        || authority.state_revision != health_receipt.authority_state_revision()
        || authority.inventory.inventory_revision != health_receipt.inventory_revision()
        || authority.inventory_digest != health_receipt.inventory_digest()
        || authority.authority_epoch != health_receipt.authority_epoch()
        || authority.process_owner_epoch != health_receipt.process_owner_epoch()
        || authority.trusted_time_high_water_ms != health_receipt.recorded_at_ms()
        || session.trusted_now_ms() >= health_receipt.expires_at_ms()
        || health_receipt.candidate_token_digest() != staging_key.candidate_token_digest()
        || health_receipt.staging_id() != staging_key.staging_id()
        || health_receipt.staging_receipt_digest() != staged.receipt().receipt_digest()
        || health_receipt.staging_run_digest() != staging_key.staging_run_digest()
        || !is_sha256(health.receipt_digest())
        || jcs_sha256_hex(health_receipt)? != health.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_HEALTH_FENCE_CHANGED");
    }
    let authority_updated_at_ms = read_authority_updated_at(transaction)?;
    if authority_updated_at_ms != authority.trusted_time_high_water_ms {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_AUTHORITY_TIME_CHANGED");
    }

    let owner = read_candidate_owner(transaction, staging_key.candidate_token())?;
    if owner.plugin_id != slot.plugin_id
        || owner.slot_ref != slot.slot_ref
        || owner.candidate_generation != staging_key.receipt_expectation().candidate_generation
        || owner.release != slot.release
        || jcs_sha256_hex(&staging_key.candidate_token())? != staging_key.candidate_token_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_OWNER_CHANGED");
    }
    validate_staged_and_health_rows(transaction, promotion, &owner)?;
    let signed_manifest_envelope_digest = read_signed_manifest_envelope_digest(
        transaction,
        &owner.owner_plan_id,
        &owner.owner_plan_digest,
        owner.application_inventory_revision,
        &owner.release,
        session.trusted_now_ms(),
    )?;
    validate_no_conflicting_work(transaction, staging_key.candidate_token())?;

    let previous_active = read_previous_active(
        transaction,
        &authority.inventory,
        &owner.plugin_id,
        &owner.slot_ref,
    )?;
    let projection = project_candidate_promotion(
        &authority,
        &owner.plugin_id,
        &owner.slot_ref,
        &owner.release,
        owner.candidate_generation,
        &owner.permission_grant_digest,
        &owner.owner_plan_id,
        &session.trusted_now,
    )?;
    session.validate_source(&guard)?;

    Ok(ComputePluginCandidatePromotionAuthorityFacts {
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
        authority_updated_at_ms_before: authority_updated_at_ms,
        promoted_at_ms: session.trusted_now_ms(),
        candidate_token_digest: staging_key.candidate_token_digest().to_string(),
        plugin_id: owner.plugin_id,
        slot_ref: owner.slot_ref,
        candidate_generation: owner.candidate_generation,
        release: owner.release,
        owner_plan_id: owner.owner_plan_id,
        owner_plan_digest: owner.owner_plan_digest,
        application_inventory_revision: owner.application_inventory_revision,
        permission_grant_digest: owner.permission_grant_digest,
        signed_manifest_envelope_digest,
        staging_id: staging_key.staging_id().to_string(),
        staging_receipt_digest: staged.receipt().receipt_digest().to_string(),
        staging_run_digest: staging_key.staging_run_digest().to_string(),
        extraction_plan_digest: staging_key
            .receipt_expectation()
            .extraction_plan_digest
            .clone(),
        extraction_evidence_digest: staging_key
            .receipt_expectation()
            .extraction_evidence_digest
            .clone(),
        staging_seal_payload_digest: staging_key
            .receipt_expectation()
            .staging_seal_payload_digest
            .clone(),
        staging_seal_file_digest: staging_key
            .receipt_expectation()
            .staging_seal_file_digest
            .clone(),
        staging_seal_identity_digest: staging_key
            .receipt_expectation()
            .staging_seal_identity_digest
            .clone(),
        health_id: health_receipt.health_id().to_string(),
        health_receipt_digest: health.receipt_digest().to_string(),
        health_observation_digest: health_receipt.health_observation_digest().to_string(),
        install_generation_before: projection.install_generation_before,
        install_generation_after: projection.install_generation_after,
        activation_generation_before: projection.activation_generation_before,
        activation_generation_after: projection.activation_generation_after,
        previous_active,
    })
}

fn read_authority_updated_at(transaction: &Transaction<'_>) -> Result<i64> {
    transaction
        .query_row(
            "SELECT updated_at_ms FROM authority_meta WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_AUTHORITY_UPDATED_AT_READ")
}

fn read_candidate_owner(
    transaction: &Transaction<'_>,
    candidate_token: &str,
) -> Result<CandidateOwnerRow> {
    type Row = (String, String, i64, String, String, String, String, i64);
    let row: Row = transaction
        .query_row(
            r#"SELECT plugin_id, slot_ref, candidate_generation, release_json,
                permission_grant_digest, owner_plan_id, owner_plan_digest,
                application_inventory_revision
            FROM candidate_owners
            WHERE candidate_token = ?1 AND state = 'owned'
              AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
              AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
            [candidate_token],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_OWNER_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_OWNER_MISSING"))?;
    Ok(CandidateOwnerRow {
        plugin_id: row.0,
        slot_ref: row.1,
        candidate_generation: row.2,
        release: serde_json::from_str(&row.3)
            .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RELEASE_PARSE")?,
        release_json: row.3,
        permission_grant_digest: row.4,
        owner_plan_id: row.5,
        owner_plan_digest: row.6,
        application_inventory_revision: row.7,
    })
}

fn validate_staged_and_health_rows(
    transaction: &Transaction<'_>,
    promotion: &RevalidatedCandidatePromotion<'_>,
    owner: &CandidateOwnerRow,
) -> Result<()> {
    let staged = promotion.staged();
    let key = staged.recovery_key();
    let health = promotion.health();
    let receipt = health.receipt();
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*)
            FROM candidate_staging_receipts AS staging
            JOIN candidate_health_receipts AS health
              ON health.staging_id = staging.staging_id
             AND health.candidate_token = staging.candidate_token
            WHERE staging.staging_id = ?1 AND staging.candidate_token = ?2
              AND staging.candidate_token_digest = ?3 AND staging.staging_run_digest = ?4
              AND staging.receipt_digest = ?5 AND health.health_id = ?6
              AND health.receipt_digest = ?7 AND health.health_observation_digest = ?8
              AND health.authority_state_revision = ?9 AND health.inventory_revision = ?10
              AND health.inventory_digest = ?11 AND health.authority_epoch = ?12
              AND health.process_owner_epoch = ?13 AND health.recorded_at_ms = ?14
              AND health.expires_at_ms = ?15"#,
            params![
                key.staging_id(),
                key.candidate_token(),
                key.candidate_token_digest(),
                key.staging_run_digest(),
                staged.receipt().receipt_digest(),
                receipt.health_id(),
                health.receipt_digest(),
                receipt.health_observation_digest(),
                receipt.authority_state_revision(),
                receipt.inventory_revision(),
                receipt.inventory_digest(),
                receipt.authority_epoch(),
                receipt.process_owner_epoch(),
                receipt.recorded_at_ms(),
                receipt.expires_at_ms(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_STAGING_HEALTH_READ")?;
    if count != 1 || owner.release_json != serde_json::to_string(&owner.release)? {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_STAGING_HEALTH_CHANGED");
    }
    Ok(())
}

fn read_signed_manifest_envelope_digest(
    transaction: &Transaction<'_>,
    plan_id: &str,
    plan_digest: &str,
    application_inventory_revision: i64,
    release: &ComputePluginReleaseRef,
    promoted_at_ms: i64,
) -> Result<String> {
    let (json, expires_at_ms): (String, i64) = transaction
        .query_row(
            r#"SELECT application.signed_manifests_json, application.expires_at_ms
            FROM plan_applications AS application
            JOIN plan_application_seals AS seal
              ON seal.plan_id = application.plan_id AND seal.plan_digest = application.plan_digest
            WHERE application.plan_id = ?1 AND application.plan_digest = ?2
              AND application.application_inventory_revision = ?3"#,
            params![plan_id, plan_digest, application_inventory_revision],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_MANIFEST_SET_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_PLAN_UNSEALED"))?;
    if promoted_at_ms >= expires_at_ms {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_PLAN_EXPIRED");
    }
    let manifests: Vec<SignedComputePluginManifest> = serde_json::from_str(&json)
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_MANIFEST_SET_PARSE")?;
    if serde_json::to_string(&manifests)? != json {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_MANIFEST_SET_CHANGED");
    }
    let mut matches = manifests.iter().filter(|signed| {
        signed.manifest.plugin_id == release.plugin_id
            && signed.manifest.plugin_version == release.plugin_version
            && signed.manifest.target.target_id == release.target_id
            && signed.manifest_digest == release.manifest_digest
            && signed.manifest.package.package_digest == release.package_digest
    });
    let signed = matches
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_MANIFEST_MISSING"))?;
    if matches.next().is_some()
        || jcs_sha256_hex(&signed.manifest)? != signed.manifest_digest
        || !is_sha256(&signed.manifest_digest)
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_MANIFEST_CHANGED");
    }
    jcs_sha256_hex(signed)
}

fn validate_no_conflicting_work(
    transaction: &Transaction<'_>,
    candidate_token: &str,
) -> Result<()> {
    let count = transaction
        .query_row(
            r#"SELECT
              (SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared')
            + (SELECT COUNT(*) FROM candidate_verification_runs WHERE state = 'prepared')
            + (SELECT COUNT(*) FROM candidate_cleanup_authorizations
               WHERE candidate_token = ?1)
            + (SELECT COUNT(*) FROM candidate_health_quarantine_receipts
               WHERE candidate_token = ?1)
            + (SELECT COUNT(*) FROM candidate_install_receipts WHERE candidate_token = ?1)
            + (SELECT COUNT(*) FROM candidate_promotion_receipts WHERE candidate_token = ?1)"#,
            [candidate_token],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_CONFLICT_READ")?;
    if count != 0 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_CONFLICTING_WORK");
    }
    Ok(())
}

fn read_previous_active(
    transaction: &Transaction<'_>,
    inventory: &crate::node_agent_compute_plugin_host::lifecycle::ComputePluginInventorySnapshot,
    plugin_id: &str,
    candidate_slot_ref: &str,
) -> Result<Option<ComputePluginPreviousActiveSlot>> {
    let record = inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == plugin_id)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECORD_MISSING"))?;
    let Some(active_slot_ref) = record.active_slot_ref.as_deref() else {
        return Ok(None);
    };
    if active_slot_ref == candidate_slot_ref
        || record.runtime.phase != RUNTIME_STOPPED
        || record.active_attempts != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_ACTIVE_NOT_QUIESCENT");
    }
    let active = record
        .slots
        .iter()
        .find(|slot| slot.slot_ref == active_slot_ref && slot.phase == SLOT_INSTALLED)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_ACTIVE_SLOT_MISSING"))?;
    let release_json = serde_json::to_string(&active.release)?;
    let row: (String, String) = transaction
        .query_row(
            r#"SELECT installation.receipt_digest, promotion.receipt_digest
            FROM candidate_promotion_receipts AS promotion
            JOIN candidate_install_receipts AS installation
              ON installation.install_id = promotion.install_id
             AND installation.receipt_digest = promotion.install_receipt_digest
            WHERE promotion.plugin_id = ?1 AND promotion.slot_ref = ?2
              AND promotion.release_json = ?3
              AND promotion.activation_generation_after = ?4"#,
            params![
                plugin_id,
                active_slot_ref,
                release_json,
                record.activation_generation
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_PREVIOUS_ACTIVE_READ")?
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_ACTIVE_RECEIPTS_MISSING")
        })?;
    Ok(Some(ComputePluginPreviousActiveSlot::new(
        active_slot_ref.to_string(),
        active.release.clone(),
        row.0,
        row.1,
    )))
}
