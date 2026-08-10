use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::readback::decode_receipt_pair;
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier,
    lifecycle::{ComputePluginLocalRecord, SLOT_INSTALLED},
    manifest_validation::is_sha256,
    plugin_manifest::SignedComputePluginManifest,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(in crate::node_agent_compute_plugin_host::local_authority) struct PlanningCandidateManifestProjection
{
    signed_manifest_envelope_digest: String,
}

impl PlanningCandidateManifestProjection {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn signed_manifest_envelope_digest(
        &self,
    ) -> &str {
        &self.signed_manifest_envelope_digest
    }
}

pub(in crate::node_agent_compute_plugin_host::local_authority) struct PlanningActivePromotionProjection
{
    install_receipt_digest: String,
    promotion_receipt_digest: String,
    signed_manifest_envelope_digest: String,
}

impl PlanningActivePromotionProjection {
    pub(in crate::node_agent_compute_plugin_host::local_authority) fn install_receipt_digest(
        &self,
    ) -> &str {
        &self.install_receipt_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn promotion_receipt_digest(
        &self,
    ) -> &str {
        &self.promotion_receipt_digest
    }

    pub(in crate::node_agent_compute_plugin_host::local_authority) fn signed_manifest_envelope_digest(
        &self,
    ) -> &str {
        &self.signed_manifest_envelope_digest
    }
}

/// Recovers the manifest envelope from the exact sealed plan that still owns the candidate.
pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_planning_candidate_manifest_on(
    transaction: &Transaction<'_>,
    installation_id_digest: &str,
    record: &ComputePluginLocalRecord,
) -> Result<Option<PlanningCandidateManifestProjection>> {
    let Some(candidate_slot_ref) = record.candidate_slot_ref.as_deref() else {
        return Ok(None);
    };
    let mut candidate_slots = record
        .slots
        .iter()
        .filter(|slot| slot.slot_ref == candidate_slot_ref);
    let candidate_slot = candidate_slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_CANDIDATE_SLOT_MISSING"))?;
    if candidate_slots.next().is_some()
        || candidate_slot.phase == SLOT_INSTALLED
        || candidate_slot.installed_at.is_some()
    {
        bail!("COMPUTE_PLUGIN_PLANNING_CANDIDATE_SLOT_INVALID");
    }
    let release_json = serde_json::to_string(&candidate_slot.release)
        .context("COMPUTE_PLUGIN_PLANNING_CANDIDATE_RELEASE_JSON")?;
    let mut statement = transaction
        .prepare(
            r#"SELECT candidate.candidate_generation,
                candidate.permission_grant_digest, candidate.owner_plan_id,
                candidate.owner_plan_digest, candidate.application_inventory_revision,
                application.signed_manifests_json, meta.inventory_revision
            FROM candidate_owners AS candidate
            JOIN plan_applications AS application
              ON application.plan_id = candidate.owner_plan_id
             AND application.plan_digest = candidate.owner_plan_digest
             AND application.application_inventory_revision =
                 candidate.application_inventory_revision
            JOIN plan_application_seals AS seal
              ON seal.plan_id = application.plan_id
             AND seal.plan_digest = application.plan_digest
             AND seal.application_request_digest = application.application_request_digest
             AND seal.receipt_digest = application.receipt_digest
            JOIN authority_meta AS meta ON meta.singleton = 1
            WHERE meta.installation_id_digest = ?1
              AND candidate.plugin_id = ?2 AND candidate.slot_ref = ?3
              AND candidate.release_json = ?4
              AND candidate.state IN ('owned', 'cleanup_pending')
            LIMIT 2"#,
        )
        .context("COMPUTE_PLUGIN_PLANNING_CANDIDATE_QUERY_PREPARE")?;
    let rows = statement
        .query_map(
            params![
                installation_id_digest,
                record.plugin_id,
                candidate_slot_ref,
                release_json,
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_PLANNING_CANDIDATE_QUERY")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("COMPUTE_PLUGIN_PLANNING_CANDIDATE_ROWS")?;
    let [(
        candidate_generation,
        permission_grant_digest,
        owner_plan_id,
        owner_plan_digest,
        application_inventory_revision,
        signed_manifests_json,
        inventory_revision,
    )] = rows.as_slice()
    else {
        if rows.is_empty() {
            bail!("COMPUTE_PLUGIN_PLANNING_CANDIDATE_OWNER_MISSING");
        }
        bail!("COMPUTE_PLUGIN_PLANNING_CANDIDATE_OWNER_AMBIGUOUS");
    };
    if *candidate_generation <= record.install_generation
        || *application_inventory_revision <= 0
        || application_inventory_revision > inventory_revision
        || !is_sha256(permission_grant_digest)
        || !is_identifier(owner_plan_id)
        || !is_sha256(owner_plan_digest)
    {
        bail!("COMPUTE_PLUGIN_PLANNING_CANDIDATE_OWNER_CHANGED");
    }
    let signed_manifest_envelope_digest = signed_manifest_envelope_digest_from_canonical_set(
        signed_manifests_json,
        &candidate_slot.release,
    )?;
    Ok(Some(PlanningCandidateManifestProjection {
        signed_manifest_envelope_digest,
    }))
}

pub(super) fn signed_manifest_envelope_digest_from_canonical_set(
    json: &str,
    release: &crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
) -> Result<String> {
    let manifests: Vec<SignedComputePluginManifest> = serde_json::from_str(json)
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

/// Reads immutable promotion provenance for the inventory's exact active slot.
///
/// `None` means either that the inventory has no active slot or that every intact receipt pair is
/// historical relative to the supplied record. A partial store, an active slot with no receipt
/// history, an ambiguous exact match, or a malformed receipt is authority corruption and fails the
/// whole projection.
pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_planning_active_promotion_on(
    transaction: &Transaction<'_>,
    installation_id_digest: &str,
    record: &ComputePluginLocalRecord,
) -> Result<Option<PlanningActivePromotionProjection>> {
    let Some(active_slot_ref) = record.active_slot_ref.as_deref() else {
        return Ok(None);
    };
    let permission_grant_digest = record
        .permission_grant_digest
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_ACTIVE_PERMISSION_MISSING"))?;
    if record.install_generation <= 0 || record.activation_generation <= 0 {
        bail!("COMPUTE_PLUGIN_PLANNING_ACTIVE_GENERATION_INVALID");
    }

    let mut active_slots = record
        .slots
        .iter()
        .filter(|slot| slot.slot_ref == active_slot_ref);
    let active_slot = active_slots
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PLANNING_ACTIVE_SLOT_MISSING"))?;
    if active_slots.next().is_some()
        || active_slot.phase != SLOT_INSTALLED
        || active_slot.installed_at.is_none()
    {
        bail!("COMPUTE_PLUGIN_PLANNING_ACTIVE_SLOT_INVALID");
    }
    let release_json = serde_json::to_string(&active_slot.release)
        .context("COMPUTE_PLUGIN_PLANNING_ACTIVE_RELEASE_JSON")?;

    validate_pair_store_integrity(transaction, installation_id_digest, &record.plugin_id)?;
    let mut statement = transaction
        .prepare(
            r#"SELECT installation.receipt_json, installation.receipt_digest,
                promotion.receipt_json, promotion.receipt_digest,
                promotion.signed_manifest_envelope_digest, owner.state,
                owner.plugin_id, owner.slot_ref, owner.release_json,
                owner.candidate_token, owner.candidate_generation,
                owner.owner_plan_id, owner.owner_plan_digest,
                owner.application_inventory_revision, owner.permission_grant_digest
            FROM candidate_install_receipts AS installation
            JOIN candidate_promotion_receipts AS promotion
              ON promotion.promotion_id = installation.promotion_id
             AND promotion.install_id = installation.install_id
             AND promotion.candidate_token = installation.candidate_token
             AND promotion.install_receipt_digest = installation.receipt_digest
            JOIN candidate_owners AS owner
              ON owner.candidate_token = installation.candidate_token
            WHERE installation.installation_id_digest = ?1
              AND promotion.installation_id_digest = ?1
              AND installation.plugin_id = ?2 AND promotion.plugin_id = ?2
              AND installation.slot_ref = ?3 AND promotion.slot_ref = ?3
              AND installation.release_json = ?4 AND promotion.release_json = ?4
              AND installation.install_generation_after = ?5
              AND promotion.install_generation_after = ?5
              AND promotion.activation_generation_after = ?6
              AND installation.permission_grant_digest = ?7
              AND promotion.permission_grant_digest = ?7
              AND installation.signed_manifest_envelope_digest =
                  promotion.signed_manifest_envelope_digest
              AND installation.install_state = 'installed'
              AND promotion.promotion_state = 'active'
            LIMIT 2"#,
        )
        .context("COMPUTE_PLUGIN_PLANNING_ACTIVE_QUERY_PREPARE")?;
    let rows = statement
        .query_map(
            params![
                installation_id_digest,
                record.plugin_id,
                active_slot_ref,
                release_json,
                record.install_generation,
                record.activation_generation,
                permission_grant_digest,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, i64>(13)?,
                    row.get::<_, String>(14)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_PLANNING_ACTIVE_QUERY")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("COMPUTE_PLUGIN_PLANNING_ACTIVE_ROWS")?;
    let [(
        install_json,
        install_digest,
        promotion_json,
        promotion_digest,
        manifest_digest,
        owner_state,
        owner_plugin_id,
        owner_slot_ref,
        owner_release_json,
        owner_candidate_token,
        owner_candidate_generation,
        owner_plan_id,
        owner_plan_digest,
        owner_application_inventory_revision,
        owner_permission_grant_digest,
    )] = rows.as_slice()
    else {
        if rows.is_empty() {
            return Ok(None);
        }
        bail!("COMPUTE_PLUGIN_PLANNING_ACTIVE_PAIR_AMBIGUOUS");
    };

    let pair = decode_receipt_pair(
        install_json,
        install_digest,
        promotion_json,
        promotion_digest,
    )?;
    let install = pair.install().receipt();
    let promotion = pair.promotion().receipt();
    if install.installation_id_digest() != installation_id_digest
        || promotion.installation_id_digest() != installation_id_digest
        || install.plugin_id() != record.plugin_id
        || promotion.plugin_id() != record.plugin_id
        || install.slot_ref() != active_slot_ref
        || promotion.slot_ref() != active_slot_ref
        || install.release() != &active_slot.release
        || promotion.release() != &active_slot.release
        || promotion.active_release_after() != &active_slot.release
        || promotion.active_slot_ref_after() != active_slot_ref
        || install.install_generation_after() != record.install_generation
        || promotion.install_generation_after() != record.install_generation
        || promotion.activation_generation_after() != record.activation_generation
        || install.permission_grant_digest() != permission_grant_digest
        || promotion.permission_grant_digest() != permission_grant_digest
        || install.signed_manifest_envelope_digest() != manifest_digest
        || promotion.signed_manifest_envelope_digest() != manifest_digest
        || install.slot_phase_after() != SLOT_INSTALLED
        || promotion.slot_phase_after() != SLOT_INSTALLED
        || owner_state != "promoted"
        || owner_plugin_id != &record.plugin_id
        || owner_slot_ref != active_slot_ref
        || owner_release_json != &release_json
        || jcs_sha256_hex(owner_candidate_token)? != install.candidate_token_digest()
        || *owner_candidate_generation != install.candidate_generation()
        || owner_plan_id != install.owner_plan_id()
        || owner_plan_digest != install.owner_plan_digest()
        || *owner_application_inventory_revision != install.application_inventory_revision()
        || owner_permission_grant_digest != install.permission_grant_digest()
    {
        bail!("COMPUTE_PLUGIN_PLANNING_ACTIVE_PAIR_CHANGED");
    }
    Ok(Some(PlanningActivePromotionProjection {
        install_receipt_digest: pair.install().receipt_digest().to_string(),
        promotion_receipt_digest: pair.promotion().receipt_digest().to_string(),
        signed_manifest_envelope_digest: manifest_digest.to_string(),
    }))
}

fn validate_pair_store_integrity(
    transaction: &Transaction<'_>,
    installation_id_digest: &str,
    plugin_id: &str,
) -> Result<()> {
    let counts = transaction
        .query_row(
            r#"SELECT
              (SELECT COUNT(*) FROM candidate_install_receipts
                WHERE installation_id_digest = ?1 AND plugin_id = ?2),
              (SELECT COUNT(*) FROM candidate_promotion_receipts
                WHERE installation_id_digest = ?1 AND plugin_id = ?2),
              (SELECT COUNT(*)
                FROM candidate_install_receipts AS installation
                JOIN candidate_promotion_receipts AS promotion
                  ON promotion.promotion_id = installation.promotion_id
                 AND promotion.install_id = installation.install_id
                 AND promotion.candidate_token = installation.candidate_token
                 AND promotion.install_receipt_digest = installation.receipt_digest
                 AND promotion.installation_id_digest = installation.installation_id_digest
                 AND promotion.candidate_token_digest = installation.candidate_token_digest
                 AND promotion.plugin_id = installation.plugin_id
                 AND promotion.slot_ref = installation.slot_ref
                 AND promotion.release_json = installation.release_json
                 AND promotion.permission_grant_digest = installation.permission_grant_digest
                 AND promotion.signed_manifest_envelope_digest =
                     installation.signed_manifest_envelope_digest
                 AND promotion.install_generation_after =
                     installation.install_generation_after
                WHERE installation.installation_id_digest = ?1
                  AND promotion.installation_id_digest = ?1
                  AND installation.plugin_id = ?2 AND promotion.plugin_id = ?2),
              (SELECT COUNT(*)
                FROM candidate_install_receipts AS installation
                JOIN candidate_owners AS owner
                  ON owner.candidate_token = installation.candidate_token
                 AND owner.plugin_id = installation.plugin_id
                 AND owner.slot_ref = installation.slot_ref
                 AND owner.release_json = installation.release_json
                 AND owner.candidate_generation = installation.candidate_generation
                 AND owner.owner_plan_id = installation.owner_plan_id
                 AND owner.owner_plan_digest = installation.owner_plan_digest
                 AND owner.application_inventory_revision =
                     installation.application_inventory_revision
                 AND owner.permission_grant_digest = installation.permission_grant_digest
                WHERE installation.installation_id_digest = ?1
                  AND installation.plugin_id = ?2)"#,
            params![installation_id_digest, plugin_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_PLANNING_ACTIVE_INTEGRITY_READ")?;
    if counts.0 <= 0 || counts.0 != counts.1 || counts.0 != counts.2 || counts.0 != counts.3 {
        bail!("COMPUTE_PLUGIN_PLANNING_ACTIVE_PAIR_STORE_CORRUPT");
    }
    Ok(())
}
