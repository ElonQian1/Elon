use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::readback::decode_receipt_pair;
use super::{
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::{
        CandidatePromotionReceiptPair, ComputePluginCandidatePromotionRecoveryKey,
        ComputePluginCandidatePromotionRecoveryOutcome,
    },
    lifecycle::{RUNTIME_STOPPED, SLOT_INSTALLED},
    local_authority::plan_application::read_authority_plan_application_state_at_or_before_observation,
    manifest_validation::is_sha256,
    trusted_time::ComputePluginTrustedTimeObservation,
};

const PROMOTION_CLOSE_REASON: &str = "candidate_promotion_completed";

pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCandidatePromotionRecoveryAuthoritySession<
    'authority,
> {
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    observed_at: Instant,
    clock_epoch_digest: String,
}

struct StoredPair {
    candidate_token: String,
    install_installation_id_digest: String,
    promotion_installation_id_digest: String,
    install_evidence_json: String,
    install_evidence_digest: String,
    install_json: String,
    install_digest: String,
    active_provenance_json: String,
    active_provenance_digest: String,
    promotion_json: String,
    promotion_digest: String,
}

impl ComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn bind_candidate_promotion_recovery_authority_session<
        'authority,
    >(
        &'authority self,
        process_fence: &'authority ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> Result<ComputePluginCandidatePromotionRecoveryAuthoritySession<'authority>> {
        let trusted_now = observation.trusted_now().clone();
        let observed_at = observation.observed_at();
        if !self
            .instance_binding()
            .matches(process_fence.authority_instance_binding())
            || !is_sha256(observation.installation_id_digest())
            || observation.installation_id_digest() != process_fence.installation_id_digest()
            || !is_sha256(observation.clock_epoch_digest())
            || observation.clock_epoch_digest() != process_fence.clock_epoch_digest()
            || process_fence.process_owner_epoch() <= 0
            || process_fence.acquired_at_ms() < 0
            || observed_at <= process_fence.acquired_observed_at()
            || trusted_now.timestamp_millis() < process_fence.acquired_at_ms()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_SESSION_INVALID");
        }
        Ok(ComputePluginCandidatePromotionRecoveryAuthoritySession {
            authority: self,
            process_fence,
            trusted_now,
            observed_at,
            clock_epoch_digest: observation.clock_epoch_digest().to_string(),
        })
    }
}

impl ComputePluginCandidatePromotionRecoveryAuthoritySession<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.process_fence.authority_instance_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.process_fence.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn process_owner_epoch(&self) -> i64 {
        self.process_fence.process_owner_epoch()
    }

    pub(in crate::node_agent_compute_plugin_host) fn clock_epoch_digest(&self) -> &str {
        &self.clock_epoch_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn read_candidate_promotion_outcome(
        &self,
        key: &ComputePluginCandidatePromotionRecoveryKey,
    ) -> Result<ComputePluginCandidatePromotionRecoveryOutcome> {
        validate_provenance(self, key)?;
        self.authority
            .with_deferred(|transaction| read_outcome(transaction, self, key))
    }
}

fn validate_provenance(
    session: &ComputePluginCandidatePromotionRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidatePromotionRecoveryKey,
) -> Result<()> {
    if !key
        .authority_instance_binding()
        .matches(session.authority_instance_binding())
        || key.installation_id_digest() != session.installation_id_digest()
        || key.clock_epoch_digest() != session.clock_epoch_digest()
        || key.expectation().process_owner_epoch() != session.process_owner_epoch()
        || session.observed_at <= session.process_fence.acquired_observed_at()
        || session.trusted_now.timestamp_millis() < key.expectation().promoted_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_PROVENANCE_CHANGED");
    }
    Ok(())
}

fn read_outcome(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidatePromotionRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidatePromotionRecoveryKey,
) -> Result<ComputePluginCandidatePromotionRecoveryOutcome> {
    let stored = read_exact_pair(transaction, key)?;
    let identity_matches = count_identity_matches(transaction, key)?;
    match stored {
        None => {
            if identity_matches != 0 {
                bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_ABSENCE_AMBIGUOUS");
            }
            Ok(ComputePluginCandidatePromotionRecoveryOutcome::NotCreated)
        }
        Some(stored) => {
            if identity_matches != 2
                || stored.install_installation_id_digest != session.installation_id_digest()
                || stored.promotion_installation_id_digest != session.installation_id_digest()
                || stored.install_evidence_json != stored.install_json
                || stored.install_evidence_digest != stored.install_digest
                || stored.active_provenance_json != stored.promotion_json
                || stored.active_provenance_digest != stored.promotion_digest
            {
                bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_RESULT_AMBIGUOUS");
            }
            let pair = decode_receipt_pair(
                &stored.install_json,
                &stored.install_digest,
                &stored.promotion_json,
                &stored.promotion_digest,
            )?;
            validate_pair(key, &pair)?;
            validate_committed_projection(transaction, session, key, &stored.candidate_token)?;
            Ok(ComputePluginCandidatePromotionRecoveryOutcome::Installed(
                pair,
            ))
        }
    }
}

fn read_exact_pair(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidatePromotionRecoveryKey,
) -> Result<Option<StoredPair>> {
    transaction
        .query_row(
            r#"SELECT installation.candidate_token,
                installation.installation_id_digest, active.installation_id_digest,
                installation.install_evidence_json, installation.install_evidence_digest,
                installation.receipt_json, installation.receipt_digest,
                active.active_provenance_json, active.active_provenance_digest,
                active.receipt_json, active.receipt_digest
            FROM candidate_install_receipts AS installation
            JOIN candidate_promotion_receipts AS active
              ON active.promotion_id = installation.promotion_id
             AND active.install_id = installation.install_id
             AND active.candidate_token = installation.candidate_token
             AND active.install_receipt_digest = installation.receipt_digest
            WHERE installation.install_id = ?1 AND active.promotion_id = ?2"#,
            params![key.install_id(), key.promotion_id()],
            |row| {
                Ok(StoredPair {
                    candidate_token: row.get(0)?,
                    install_installation_id_digest: row.get(1)?,
                    promotion_installation_id_digest: row.get(2)?,
                    install_evidence_json: row.get(3)?,
                    install_evidence_digest: row.get(4)?,
                    install_json: row.get(5)?,
                    install_digest: row.get(6)?,
                    active_provenance_json: row.get(7)?,
                    active_provenance_digest: row.get(8)?,
                    promotion_json: row.get(9)?,
                    promotion_digest: row.get(10)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_READ")
}

fn count_identity_matches(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidatePromotionRecoveryKey,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT
              (SELECT COUNT(*) FROM candidate_install_receipts
               WHERE install_id = ?1 OR promotion_id = ?2 OR candidate_token_digest = ?3
                  OR receipt_digest = ?4)
              +
              (SELECT COUNT(*) FROM candidate_promotion_receipts
               WHERE install_id = ?1 OR promotion_id = ?2 OR candidate_token_digest = ?3
                  OR receipt_digest = ?5 OR install_receipt_digest = ?4)"#,
            params![
                key.install_id(),
                key.promotion_id(),
                key.candidate_token_digest(),
                key.expectation().expected_install_receipt_digest(),
                key.expectation().expected_promotion_receipt_digest(),
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_IDENTITY_READ")
}

fn validate_pair(
    key: &ComputePluginCandidatePromotionRecoveryKey,
    pair: &CandidatePromotionReceiptPair,
) -> Result<()> {
    let install = pair.install();
    let promotion = pair.promotion();
    let install_body = install.receipt();
    let promotion_body = promotion.receipt();
    let expected = key.expectation();
    if install.receipt_digest() != expected.expected_install_receipt_digest()
        || promotion.receipt_digest() != expected.expected_promotion_receipt_digest()
        || install_body.install_receipt_id() != key.install_id()
        || install_body.promotion_id() != key.promotion_id()
        || install_body.installation_id_digest() != key.installation_id_digest()
        || install_body.candidate_token_digest() != key.candidate_token_digest()
        || install_body.plugin_id() != key.plugin_id()
        || install_body.slot_ref() != key.slot_ref()
        || install_body.release() != key.release()
        || install_body.candidate_generation() != expected.candidate_generation()
        || install_body.staging_receipt_digest() != expected.staging_receipt_digest()
        || install_body.health_receipt_digest() != expected.health_receipt_digest()
        || install_body.owner_plan_id() != expected.owner_plan_id()
        || install_body.owner_plan_digest() != expected.owner_plan_digest()
        || install_body.application_inventory_revision()
            != expected.application_inventory_revision()
        || install_body.permission_grant_digest() != expected.permission_grant_digest()
        || install_body.signed_manifest_envelope_digest()
            != expected.signed_manifest_envelope_digest()
        || !revision_expectation_matches(key, install_body, promotion_body)
        || !previous_active_matches(key, promotion_body)
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_RECEIPT_CHANGED");
    }
    Ok(())
}

fn revision_expectation_matches(
    key: &ComputePluginCandidatePromotionRecoveryKey,
    install: &crate::node_agent_compute_plugin_host::candidate_promotion_contract::ComputePluginInstallReceipt,
    promotion: &crate::node_agent_compute_plugin_host::candidate_promotion_contract::ComputePluginPromotionReceipt,
) -> bool {
    let expected = key.expectation();
    install.authority_state_revision_before() == expected.authority_state_revision_before()
        && install.authority_state_revision_after() == expected.authority_state_revision_after()
        && install.inventory_revision_before() == expected.inventory_revision_before()
        && install.inventory_revision_after() == expected.inventory_revision_after()
        && install.inventory_digest_before() == expected.inventory_digest_before()
        && install.inventory_digest_after() == expected.inventory_digest_after()
        && install.authority_epoch_before() == expected.authority_epoch_before()
        && install.authority_epoch_after() == expected.authority_epoch_after()
        && install.process_owner_epoch() == expected.process_owner_epoch()
        && install.trusted_time_high_water_ms_before()
            == expected.trusted_time_high_water_ms_before()
        && install.authority_updated_at_ms_before() == expected.authority_updated_at_ms_before()
        && install.installed_at_ms() == expected.promoted_at_ms()
        && install.install_generation_before() == expected.install_generation_before()
        && install.install_generation_after() == expected.install_generation_after()
        && promotion.activation_generation_before() == expected.activation_generation_before()
        && promotion.activation_generation_after() == expected.activation_generation_after()
}

fn previous_active_matches(
    key: &ComputePluginCandidatePromotionRecoveryKey,
    receipt: &crate::node_agent_compute_plugin_host::candidate_promotion_contract::ComputePluginPromotionReceipt,
) -> bool {
    let expected = key.expectation();
    receipt.previous_active_slot_ref() == expected.previous_active_slot_ref()
        && receipt.previous_active_release() == expected.previous_active_release()
        && receipt.previous_active_install_receipt_digest()
            == expected.previous_active_install_receipt_digest()
        && receipt.previous_active_promotion_receipt_digest()
            == expected.previous_active_promotion_receipt_digest()
}

fn validate_committed_projection(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidatePromotionRecoveryAuthoritySession<'_>,
    key: &ComputePluginCandidatePromotionRecoveryKey,
    candidate_token: &str,
) -> Result<()> {
    let expected = key.expectation();
    let release_json = serde_json::to_string(key.release())?;
    let owner_count = transaction.query_row(
        r#"SELECT COUNT(*) FROM candidate_owners
        WHERE candidate_token = ?1 AND plugin_id = ?2 AND slot_ref = ?3
          AND candidate_generation = ?4 AND owner_plan_id = ?5 AND owner_plan_digest = ?6
          AND application_inventory_revision = ?7 AND permission_grant_digest = ?8
          AND release_json = ?9 AND state = 'promoted' AND closed_at_ms = ?10
          AND close_reason = ?11
          AND closed_by_plan_id IS NULL AND closed_by_plan_digest IS NULL"#,
        params![
            candidate_token,
            key.plugin_id(),
            key.slot_ref(),
            expected.candidate_generation(),
            expected.owner_plan_id(),
            expected.owner_plan_digest(),
            expected.application_inventory_revision(),
            expected.permission_grant_digest(),
            release_json,
            expected.promoted_at_ms(),
            PROMOTION_CLOSE_REASON,
        ],
        |row| row.get::<_, i64>(0),
    )?;
    let authority = read_authority_plan_application_state_at_or_before_observation(
        transaction,
        &session.trusted_now,
    )?;
    let record = authority
        .inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == key.plugin_id());
    let slot = record.and_then(|record| {
        record
            .slots
            .iter()
            .find(|slot| slot.slot_ref == key.slot_ref() && &slot.release == key.release())
    });
    if owner_count != 1
        || authority.installation_id_digest != key.installation_id_digest()
        || authority.state_revision != expected.authority_state_revision_after()
        || authority.inventory.inventory_revision != expected.inventory_revision_after()
        || authority.inventory_digest != expected.inventory_digest_after()
        || authority.authority_epoch != expected.authority_epoch_after()
        || authority.process_owner_epoch != expected.process_owner_epoch()
        || authority.trusted_time_high_water_ms != expected.promoted_at_ms()
        || record.is_none_or(|record| {
            record.install_generation != expected.install_generation_after()
                || record.activation_generation != expected.activation_generation_after()
                || record.active_slot_ref.as_deref() != Some(key.slot_ref())
                || record.candidate_slot_ref.is_some()
                || record.permission_grant_digest.as_deref()
                    != Some(expected.permission_grant_digest())
                || record.runtime.phase != RUNTIME_STOPPED
                || record.active_attempts != 0
                || record.health.is_some()
        })
        || slot.is_none_or(|slot| slot.phase != SLOT_INSTALLED || slot.installed_at.is_none())
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_RECOVERY_PROJECTION_CHANGED");
    }
    Ok(())
}
