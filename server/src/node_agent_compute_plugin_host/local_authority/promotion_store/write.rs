use std::time::Instant;

use anyhow::{bail, Context, Result};
use rusqlite::{named_params, params, Transaction};

use super::{
    binding::read_candidate_promotion_binding, meta::update_promotion_authority_meta,
    projection::project_candidate_promotion, readback::decode_receipt_pair,
    ComputePluginCandidatePromotionAuthorityFacts,
    ComputePluginPostRevalidationPromotionAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::{
        HashedComputePluginInstallReceipt, HashedComputePluginPromotionReceipt,
        ValidatedCandidatePromotionStorePermit,
    },
    lifecycle::{RUNTIME_STOPPED, SLOT_INSTALLED},
    local_authority::{
        keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
        plan_application::{
            read_authority_plan_application_state,
            read_authority_plan_application_state_at_or_before_observation,
        },
    },
};

const PROMOTION_CLOSE_REASON: &str = "candidate_promotion_completed";

pub(super) fn persist_candidate_promotion(
    transaction: &Transaction<'_>,
    session: &ComputePluginPostRevalidationPromotionAuthoritySession<'_>,
    permit: ValidatedCandidatePromotionStorePermit<'_, '_>,
) -> Result<()> {
    let revalidated = permit.revalidated();
    revalidated.trusted_time().ensure_live(Instant::now())?;
    let guard = revalidated.staged().archive().snapshot_cancellation_guard();
    session.validate_source(&guard)?;
    let current = read_candidate_promotion_binding(transaction, session, revalidated)?;
    if &current != permit.facts() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_AUTHORITY_CHANGED");
    }
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    let projection = project_candidate_promotion(
        &authority,
        current.plugin_id(),
        current.slot_ref(),
        current.release(),
        current.candidate_generation(),
        current.permission_grant_digest(),
        current.owner_plan_id(),
        &session.trusted_now,
    )?;
    validate_projection(&current, &projection)?;
    permit.receipts().validate()?;
    let install = permit.receipts().install();
    let promotion = permit.receipts().promotion();
    insert_install_receipt(transaction, &permit, &current, &projection, &install)?;
    insert_promotion_receipt(
        transaction,
        &permit,
        &current,
        &projection,
        &install,
        &promotion,
    )?;
    session.validate_source(&guard)?;

    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != current.authority_state_revision_before()
        || time_state.authority_epoch != current.authority_epoch_before()
        || time_state.trusted_time_high_water_ms
            != Some(current.trusted_time_high_water_ms_before())
        || time_state.clock_status != "trusted"
        || current.promoted_at_ms() <= current.trusted_time_high_water_ms_before()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, current.promoted_at_ms())?;
    update_promotion_authority_meta(
        transaction,
        &authority,
        &projection,
        current.promoted_at_ms(),
    )?;
    promote_candidate_owner(transaction, &permit, &current)?;
    validate_readback(
        transaction,
        session,
        &permit,
        &current,
        &install,
        &promotion,
    )?;
    session.validate_source(&guard)?;
    permit
        .revalidated()
        .trusted_time()
        .ensure_live(Instant::now())?;
    Ok(())
}

fn validate_projection(
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
    projection: &super::projection::CandidatePromotionProjection,
) -> Result<()> {
    if projection.state_revision != facts.authority_state_revision_after()
        || projection.inventory.inventory_revision != facts.inventory_revision_after()
        || projection.inventory_digest != facts.inventory_digest_after()
        || projection.authority_epoch != facts.authority_epoch_after()
        || projection.install_generation_before != facts.install_generation_before()
        || projection.install_generation_after != facts.install_generation_after()
        || projection.activation_generation_before != facts.activation_generation_before()
        || projection.activation_generation_after != facts.activation_generation_after()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_PROJECTION_CHANGED");
    }
    Ok(())
}

fn insert_install_receipt(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidatePromotionStorePermit<'_, '_>,
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
    projection: &super::projection::CandidatePromotionProjection,
    hashed: &HashedComputePluginInstallReceipt,
) -> Result<()> {
    let key = permit.revalidated().staged().recovery_key();
    let release_json = serde_json::to_string(facts.release())?;
    let receipt_json = serde_json::to_string(hashed.receipt())?;
    transaction.execute(
        r#"INSERT INTO candidate_install_receipts (
          install_id, promotion_id, installation_id_digest, candidate_token, candidate_token_digest,
          plugin_id, slot_ref, candidate_generation, owner_plan_id, owner_plan_digest,
          application_inventory_revision, staging_id, staging_receipt_digest,
          staging_run_digest, health_id, health_receipt_digest, health_observation_digest,
          release_json, permission_grant_digest, signed_manifest_envelope_digest,
          install_state, install_evidence_json, install_evidence_digest,
          install_generation_before, install_generation_after,
          authority_state_revision_before, authority_state_revision_after,
          inventory_revision_before, inventory_revision_after, inventory_digest_before,
          inventory_digest_after, inventory_json_after, authority_epoch_before,
          authority_epoch_after, process_owner_epoch, trusted_time_before_ms,
          authority_updated_at_ms_before, installed_at_ms, receipt_json, receipt_digest
        ) VALUES (
          :install_id, :promotion_id, :installation_id_digest, :candidate_token, :candidate_token_digest,
          :plugin_id, :slot_ref, :candidate_generation, :owner_plan_id, :owner_plan_digest,
          :application_inventory_revision, :staging_id, :staging_receipt_digest,
          :staging_run_digest, :health_id, :health_receipt_digest, :health_observation_digest,
          :release_json, :permission_grant_digest, :manifest_digest,
          'installed', :receipt_json, :receipt_digest, :install_before, :install_after,
          :state_before, :state_after, :inventory_before, :inventory_after,
          :inventory_digest_before, :inventory_digest_after, :inventory_json_after,
          :epoch_before, :epoch_after, :process_epoch, :trusted_before,
          :updated_before, :installed_at, :receipt_json, :receipt_digest
        )"#,
        named_params! {
            ":install_id": permit.install_id(), ":promotion_id": permit.promotion_id(),
            ":installation_id_digest": hashed.receipt().installation_id_digest(),
            ":candidate_token": key.candidate_token(),
            ":candidate_token_digest": facts.candidate_token_digest(),
            ":plugin_id": facts.plugin_id(), ":slot_ref": facts.slot_ref(),
            ":candidate_generation": facts.candidate_generation(),
            ":owner_plan_id": facts.owner_plan_id(), ":owner_plan_digest": facts.owner_plan_digest(),
            ":application_inventory_revision": facts.application_inventory_revision(),
            ":staging_id": facts.staging_id(), ":staging_receipt_digest": facts.staging_receipt_digest(),
            ":staging_run_digest": facts.staging_run_digest(), ":health_id": facts.health_id(),
            ":health_receipt_digest": facts.health_receipt_digest(),
            ":health_observation_digest": facts.health_observation_digest(),
            ":release_json": &release_json, ":permission_grant_digest": facts.permission_grant_digest(),
            ":manifest_digest": facts.signed_manifest_envelope_digest(),
            ":receipt_json": &receipt_json, ":receipt_digest": hashed.receipt_digest(),
            ":install_before": facts.install_generation_before(),
            ":install_after": facts.install_generation_after(),
            ":state_before": facts.authority_state_revision_before(),
            ":state_after": facts.authority_state_revision_after(),
            ":inventory_before": facts.inventory_revision_before(),
            ":inventory_after": facts.inventory_revision_after(),
            ":inventory_digest_before": facts.inventory_digest_before(),
            ":inventory_digest_after": facts.inventory_digest_after(),
            ":inventory_json_after": &projection.inventory_json,
            ":epoch_before": facts.authority_epoch_before(), ":epoch_after": facts.authority_epoch_after(),
            ":process_epoch": facts.process_owner_epoch(),
            ":trusted_before": facts.trusted_time_high_water_ms_before(),
            ":updated_before": facts.authority_updated_at_ms_before(),
            ":installed_at": facts.promoted_at_ms(),
        },
    ).context("COMPUTE_PLUGIN_CANDIDATE_INSTALL_RECEIPT_INSERT")?;
    Ok(())
}

fn insert_promotion_receipt(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidatePromotionStorePermit<'_, '_>,
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
    projection: &super::projection::CandidatePromotionProjection,
    install: &HashedComputePluginInstallReceipt,
    promotion: &HashedComputePluginPromotionReceipt,
) -> Result<()> {
    let key = permit.revalidated().staged().recovery_key();
    let release_json = serde_json::to_string(facts.release())?;
    let previous_release_json = facts
        .previous_active()
        .map(|previous| serde_json::to_string(previous.release()))
        .transpose()?;
    let receipt_json = serde_json::to_string(promotion.receipt())?;
    transaction.execute(
        r#"INSERT INTO candidate_promotion_receipts (
          promotion_id, install_id, install_receipt_digest, installation_id_digest, candidate_token,
          candidate_token_digest, plugin_id, slot_ref, candidate_generation,
          owner_plan_id, owner_plan_digest, application_inventory_revision,
          staging_id, staging_receipt_digest, health_id, health_receipt_digest,
          release_json, permission_grant_digest, signed_manifest_envelope_digest,
          promotion_state, active_provenance_json, active_provenance_digest,
          install_generation_after, activation_generation_before, activation_generation_after,
          previous_active_slot_ref, previous_active_release_json,
          previous_active_install_receipt_digest, previous_active_promotion_receipt_digest,
          authority_state_revision_before, authority_state_revision_after,
          inventory_revision_before, inventory_revision_after, inventory_digest_before,
          inventory_digest_after, inventory_json_after, authority_epoch_before,
          authority_epoch_after, process_owner_epoch, trusted_time_before_ms,
          authority_updated_at_ms_before, installed_at_ms, promoted_at_ms,
          close_reason, receipt_json, receipt_digest
        ) VALUES (
          :promotion_id, :install_id, :install_digest, :installation_id_digest, :candidate_token,
          :candidate_token_digest, :plugin_id, :slot_ref, :candidate_generation,
          :owner_plan_id, :owner_plan_digest, :application_inventory_revision,
          :staging_id, :staging_receipt_digest, :health_id, :health_receipt_digest,
          :release_json, :permission_grant_digest, :manifest_digest,
          'active', :receipt_json, :receipt_digest, :install_after,
          :activation_before, :activation_after, :previous_slot, :previous_release,
          :previous_install, :previous_promotion, :state_before, :state_after,
          :inventory_before, :inventory_after, :inventory_digest_before,
          :inventory_digest_after, :inventory_json_after, :epoch_before, :epoch_after,
          :process_epoch, :trusted_before, :updated_before, :installed_at, :promoted_at,
          :close_reason, :receipt_json, :receipt_digest
        )"#,
        named_params! {
            ":promotion_id": permit.promotion_id(), ":install_id": permit.install_id(),
            ":install_digest": install.receipt_digest(), ":candidate_token": key.candidate_token(),
            ":installation_id_digest": promotion.receipt().installation_id_digest(),
            ":candidate_token_digest": facts.candidate_token_digest(),
            ":plugin_id": facts.plugin_id(), ":slot_ref": facts.slot_ref(),
            ":candidate_generation": facts.candidate_generation(),
            ":owner_plan_id": facts.owner_plan_id(), ":owner_plan_digest": facts.owner_plan_digest(),
            ":application_inventory_revision": facts.application_inventory_revision(),
            ":staging_id": facts.staging_id(), ":staging_receipt_digest": facts.staging_receipt_digest(),
            ":health_id": facts.health_id(), ":health_receipt_digest": facts.health_receipt_digest(),
            ":release_json": &release_json, ":permission_grant_digest": facts.permission_grant_digest(),
            ":manifest_digest": facts.signed_manifest_envelope_digest(),
            ":receipt_json": &receipt_json, ":receipt_digest": promotion.receipt_digest(),
            ":install_after": facts.install_generation_after(),
            ":activation_before": facts.activation_generation_before(),
            ":activation_after": facts.activation_generation_after(),
            ":previous_slot": facts.previous_active().map(|value| value.slot_ref()),
            ":previous_release": previous_release_json.as_deref(),
            ":previous_install": facts.previous_active().map(|value| value.install_receipt_digest()),
            ":previous_promotion": facts.previous_active().map(|value| value.promotion_receipt_digest()),
            ":state_before": facts.authority_state_revision_before(),
            ":state_after": facts.authority_state_revision_after(),
            ":inventory_before": facts.inventory_revision_before(),
            ":inventory_after": facts.inventory_revision_after(),
            ":inventory_digest_before": facts.inventory_digest_before(),
            ":inventory_digest_after": facts.inventory_digest_after(),
            ":inventory_json_after": &projection.inventory_json,
            ":epoch_before": facts.authority_epoch_before(), ":epoch_after": facts.authority_epoch_after(),
            ":process_epoch": facts.process_owner_epoch(),
            ":trusted_before": facts.trusted_time_high_water_ms_before(),
            ":updated_before": facts.authority_updated_at_ms_before(),
            ":installed_at": facts.promoted_at_ms(), ":promoted_at": facts.promoted_at_ms(),
            ":close_reason": PROMOTION_CLOSE_REASON,
        },
    ).context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECEIPT_INSERT")?;
    Ok(())
}

fn promote_candidate_owner(
    transaction: &Transaction<'_>,
    permit: &ValidatedCandidatePromotionStorePermit<'_, '_>,
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
) -> Result<()> {
    let changed = transaction.execute(
        r#"UPDATE candidate_owners SET state = 'promoted', closed_at_ms = :promoted_at,
          closed_by_plan_id = NULL, closed_by_plan_digest = NULL, close_reason = :reason
        WHERE candidate_token = :candidate_token AND plugin_id = :plugin_id
          AND slot_ref = :slot_ref AND candidate_generation = :candidate_generation
          AND owner_plan_id = :owner_plan_id AND owner_plan_digest = :owner_plan_digest
          AND application_inventory_revision = :application_inventory_revision
          AND release_json = :release_json AND permission_grant_digest = :permission_grant_digest
          AND state = 'owned' AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
          AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
        named_params! {
            ":promoted_at": facts.promoted_at_ms(), ":reason": PROMOTION_CLOSE_REASON,
            ":candidate_token": permit.revalidated().staged().recovery_key().candidate_token(),
            ":plugin_id": facts.plugin_id(), ":slot_ref": facts.slot_ref(),
            ":candidate_generation": facts.candidate_generation(),
            ":owner_plan_id": facts.owner_plan_id(), ":owner_plan_digest": facts.owner_plan_digest(),
            ":application_inventory_revision": facts.application_inventory_revision(),
            ":release_json": serde_json::to_string(facts.release())?,
            ":permission_grant_digest": facts.permission_grant_digest(),
        },
    ).context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_OWNER_UPDATE")?;
    if changed != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_OWNER_CAS");
    }
    Ok(())
}

fn validate_readback(
    transaction: &Transaction<'_>,
    session: &ComputePluginPostRevalidationPromotionAuthoritySession<'_>,
    permit: &ValidatedCandidatePromotionStorePermit<'_, '_>,
    facts: &ComputePluginCandidatePromotionAuthorityFacts,
    install: &HashedComputePluginInstallReceipt,
    promotion: &HashedComputePluginPromotionReceipt,
) -> Result<()> {
    let install_json = serde_json::to_string(install.receipt())?;
    let promotion_json = serde_json::to_string(promotion.receipt())?;
    let count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_install_receipts AS installation
        JOIN candidate_promotion_receipts AS active
          ON active.promotion_id = installation.promotion_id
         AND active.install_id = installation.install_id
         AND active.candidate_token = installation.candidate_token
         AND active.install_receipt_digest = installation.receipt_digest
        JOIN candidate_owners AS owner ON owner.candidate_token = installation.candidate_token
        WHERE installation.install_id = ?1 AND active.promotion_id = ?2
          AND installation.candidate_token_digest = ?3
          AND installation.receipt_json = ?4 AND installation.receipt_digest = ?5
          AND active.receipt_json = ?6 AND active.receipt_digest = ?7
          AND installation.install_evidence_json = installation.receipt_json
          AND installation.install_evidence_digest = installation.receipt_digest
          AND active.active_provenance_json = active.receipt_json
          AND active.active_provenance_digest = active.receipt_digest
          AND installation.installation_id_digest = active.installation_id_digest
          AND installation.installation_id_digest = ?10
          AND owner.state = 'promoted' AND owner.closed_at_ms = ?8
          AND owner.close_reason = ?9"#,
            params![
                permit.install_id(),
                permit.promotion_id(),
                facts.candidate_token_digest(),
                install_json,
                install.receipt_digest(),
                promotion_json,
                promotion.receipt_digest(),
                facts.promoted_at_ms(),
                PROMOTION_CLOSE_REASON,
                session.installation_id_digest(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECEIPT_READBACK")?;
    if count != 1 {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECEIPT_READBACK_CHANGED");
    }
    let pair = decode_receipt_pair(
        &serde_json::to_string(install.receipt())?,
        install.receipt_digest(),
        &serde_json::to_string(promotion.receipt())?,
        promotion.receipt_digest(),
    )?;
    if pair.install().receipt_digest() != install.receipt_digest()
        || pair.promotion().receipt_digest() != promotion.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECEIPT_READBACK_CHANGED");
    }
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    let record = authority
        .inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == facts.plugin_id())
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_READBACK_MISSING"))?;
    let slot = record
        .slots
        .iter()
        .find(|slot| slot.slot_ref == facts.slot_ref() && &slot.release == facts.release());
    if authority.state_revision != facts.authority_state_revision_after()
        || authority.inventory.inventory_revision != facts.inventory_revision_after()
        || authority.inventory_digest != facts.inventory_digest_after()
        || authority.authority_epoch != facts.authority_epoch_after()
        || authority.process_owner_epoch != facts.process_owner_epoch()
        || authority.trusted_time_high_water_ms != facts.promoted_at_ms()
        || record.install_generation != facts.install_generation_after()
        || record.activation_generation != facts.activation_generation_after()
        || record.active_slot_ref.as_deref() != Some(facts.slot_ref())
        || record.candidate_slot_ref.is_some()
        || record.permission_grant_digest.as_deref() != Some(facts.permission_grant_digest())
        || record.runtime.phase != RUNTIME_STOPPED
        || record.active_attempts != 0
        || record.health.is_some()
        || slot.is_none_or(|slot| slot.phase != SLOT_INSTALLED || slot.installed_at.is_none())
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_AUTHORITY_READBACK_CHANGED");
    }
    Ok(())
}
