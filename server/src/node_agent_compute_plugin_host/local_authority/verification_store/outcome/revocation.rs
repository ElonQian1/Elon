use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{VerificationRecoveryAuthorityRow, VerificationRunRow};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    identity::ComputePluginReleaseRef,
    lifecycle::{ComputePluginInventorySnapshot, SLOT_REMOVING},
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn validate_durable_state(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    run: &VerificationRunRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    reason: &str,
) -> Result<()> {
    match reason {
        "authority_epoch_advanced_by_keyring" | "authority_epoch_advanced_by_plan" => {
            if authority.authority_epoch <= key.authority_epoch() {
                bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_REVOKE_FENCE_MISSING");
            }
        }
        "authority_epoch_advanced_by_verification" => {
            if authority.authority_epoch <= key.authority_epoch()
                || authority.inventory_revision <= key.execution_inventory_revision()
            {
                bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_REVOKE_FENCE_MISSING");
            }
        }
        "process_owner_epoch_advanced" => {
            if authority.process_owner_epoch <= key.process_owner_epoch() {
                bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_REVOKE_FENCE_MISSING");
            }
        }
        "candidate_released_by_plan" => {
            validate_candidate_release(transaction, authority, run, key)?;
        }
        _ => bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_REVOKE_REASON_CORRUPT"),
    }
    Ok(())
}

fn validate_candidate_release(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    run: &VerificationRunRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<()> {
    type CandidateRow = (
        String,
        String,
        i64,
        String,
        String,
        i64,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    );
    let candidate: CandidateRow = transaction
        .query_row(
            r#"SELECT plugin_id, slot_ref, candidate_generation, owner_plan_id,
                owner_plan_digest, application_inventory_revision, state, closed_at_ms,
                closed_by_plan_id, closed_by_plan_digest, close_reason, release_json
            FROM candidate_owners WHERE candidate_token = ?1"#,
            [key.candidate_token()],
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
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_CANDIDATE_READ")?
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_CANDIDATE_MISSING")
        })?;
    let resolved_at_ms = run.resolved_at_ms.ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_TIME_MISSING")
    })?;
    let release: ComputePluginReleaseRef = serde_json::from_str(&candidate.11)
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_PARSE")?;
    if candidate.2 != key.candidate_generation()
        || candidate.3 != key.owner_plan_id()
        || candidate.4 != key.owner_plan_digest()
        || candidate.5 != key.application_inventory_revision()
        || candidate.6 != "released"
        || candidate.7 != Some(resolved_at_ms)
        || candidate.8.is_none()
        || candidate.9.is_none()
        || candidate.10.as_deref() != Some("cancel_candidate")
        || serde_json::to_string(&release)? != candidate.11
        || authority.authority_epoch <= key.authority_epoch()
        || authority.inventory_revision <= key.execution_inventory_revision()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_STATE_CHANGED");
    }
    let sealed = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM plan_application_seals AS seal
            JOIN plan_applications AS application
              ON application.plan_id = seal.plan_id
             AND application.plan_digest = seal.plan_digest
            WHERE seal.plan_id = ?1 AND seal.plan_digest = ?2
              AND application.applied_at_ms = ?3"#,
            params![
                candidate.8.as_deref(),
                candidate.9.as_deref(),
                resolved_at_ms
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_PLAN_READ")?;
    let inventory: ComputePluginInventorySnapshot = serde_json::from_str(&authority.inventory_json)
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_INVENTORY_PARSE")?;
    let record_matches = inventory.plugins.iter().any(|record| {
        record.plugin_id == candidate.0
            && record.last_plan_id.as_deref() == candidate.8.as_deref()
            && record.candidate_slot_ref.is_none()
            && record.slots.iter().any(|slot| {
                slot.slot_ref == candidate.1
                    && slot.release == release
                    && slot.phase == SLOT_REMOVING
            })
    });
    if sealed != 1
        || inventory.inventory_revision != authority.inventory_revision
        || serde_json::to_string(&inventory)? != authority.inventory_json
        || jcs_sha256_hex(&inventory)? != authority.inventory_digest
        || !record_matches
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RELEASE_INVENTORY_CHANGED");
    }
    Ok(())
}
