use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::VerificationRecoveryAuthorityRow;
use crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey;

use super::super::closure::{self, CandidateClosureSnapshot};

pub(super) fn validate_not_created(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<CandidateClosureSnapshot> {
    let before = key
        .initial_absence()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RUN_MISSING"))?;
    if before.authority_state_revision() != key.authority_state_revision()
        || before.inventory_revision() != key.execution_inventory_revision()
        || before.inventory_digest() != key.inventory_digest()
        || before.next_verification_generation() != key.verification_generation()
        || before.durable_candidate_closure_digest() != key.durable_candidate_closure_digest()
        || authority.state_revision != before.authority_state_revision()
        || authority.inventory_revision != before.inventory_revision()
        || authority.inventory_digest != before.inventory_digest()
        || authority.authority_epoch != key.authority_epoch()
        || authority.process_owner_epoch != key.process_owner_epoch()
        || authority.trusted_time_high_water_ms != before.trusted_time_high_water_ms()
        || authority.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_NOT_CREATED_AUTHORITY_CHANGED");
    }
    let sealed = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM plan_application_seals
        WHERE plan_id = ?1 AND plan_digest = ?2"#,
            params![key.owner_plan_id(), key.owner_plan_digest()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_NOT_CREATED_PLAN_READ")?;
    if sealed != 1 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_NOT_CREATED_PLAN_CHANGED");
    }
    let closure = closure::read_candidate_closure_snapshot(transaction, key.candidate_token())?;
    if closure.durable_closure_digest != before.durable_candidate_closure_digest()
        || closure.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || closure.artifact_count != key.artifact_count()
        || closure.artifact_bytes != key.artifact_bytes()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_NOT_CREATED_CLOSURE_CHANGED");
    }
    let (last_generation, open_count) = transaction
        .query_row(
            r#"SELECT COALESCE(MAX(verification_generation), 0),
            COALESCE(SUM(CASE WHEN state IN ('prepared', 'verified') THEN 1 ELSE 0 END), 0)
        FROM candidate_verification_runs WHERE candidate_token = ?1"#,
            [key.candidate_token()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_NOT_CREATED_GENERATION_READ")?;
    let expected_generation = last_generation.checked_add(1).ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_GENERATION_EXHAUSTED")
    })?;
    let prepared_fetches = transaction
        .query_row(
            "SELECT COUNT(*) FROM fetch_claims WHERE candidate_token = ?1 AND state = 'prepared'",
            [key.candidate_token()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_NOT_CREATED_FETCH_READ")?;
    if expected_generation != key.verification_generation()
        || open_count != 0
        || prepared_fetches != 0
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_NOT_CREATED_CONFLICT");
    }
    Ok(closure)
}
