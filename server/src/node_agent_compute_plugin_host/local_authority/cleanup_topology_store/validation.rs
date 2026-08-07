use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use super::ComputePluginCandidateCleanupTopologyAuthoritySession;
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        restore_hashed_execution_plan, restore_hashed_expected_object,
        validate_hashed_execution_plan, CandidateCleanupExecutionState,
        ComputePluginCandidateCleanupExecutionPlan, ComputePluginCandidateCleanupExpectedObject,
        HashedCandidateCleanupExpectedObject, HashedComputePluginCandidateCleanupExecutionPlan,
    },
    local_authority::plan_application::read_authority_plan_application_state,
    signed_artifact_verification::jcs_sha256_hex,
};

pub(super) fn validate_unsealed_binding(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupTopologyAuthoritySession<'_>,
    state: &CandidateCleanupExecutionState,
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
) -> Result<()> {
    validate_hashed_execution_plan(plan)?;
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let recovery = state.staging_recovery_key();
    let candidate_parent_anchor_identity_digest =
        state.candidate_parent_anchor_identity_digest()?;
    if plan.plan().cleanup_id() != receipt.cleanup_id()
        || plan.plan().candidate_token_digest() != receipt.candidate_token_digest()
        || plan.plan().authorization_receipt_digest() != authorization.receipt_digest()
        || plan.plan().installation_id_digest() != session.installation_id_digest()
        || plan.plan().root_identity_digest() != recovery.root_identity_digest()
        || plan.plan().candidate_parent_anchor_identity_digest()
            != candidate_parent_anchor_identity_digest
        || plan.plan().process_owner_epoch() != session.process_owner_epoch()
        || plan.plan().planned_at_ms() != session.trusted_now_ms()
        || plan.plan().planned_at_ms() <= receipt.authorized_at_ms()
        || jcs_sha256_hex(receipt)? != authorization.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_BINDING_CHANGED");
    }
    validate_authority_and_owner(transaction, session, state, receipt.authorized_at_ms())?;
    if count_plan_identity_matches(transaction, recovery.candidate_token(), plan)? != 0
        || count_objects(transaction, receipt.cleanup_id())? != 0
        || count_seals(transaction, receipt.cleanup_id())? != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_ALREADY_EXISTS");
    }
    Ok(())
}

pub(super) fn validate_authority_and_owner(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupTopologyAuthoritySession<'_>,
    state: &CandidateCleanupExecutionState,
    expected_high_water_ms: i64,
) -> Result<()> {
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let recovery = state.staging_recovery_key();
    let authority = read_authority_plan_application_state(transaction, &session.trusted_now)?;
    if authority.installation_id_digest != session.installation_id_digest()
        || authority.process_owner_epoch != session.process_owner_epoch()
        || authority.state_revision != receipt.authority_state_revision_after()
        || authority.inventory.inventory_revision != receipt.inventory_revision()
        || authority.inventory_digest != receipt.inventory_digest()
        || authority.authority_epoch != receipt.authority_epoch_after()
        || authority.trusted_time_high_water_ms != expected_high_water_ms
        || count_exact_authorization(transaction, state)? != 1
        || count_pending_owner(transaction, recovery.candidate_token())? != 1
        || count_completion(transaction, recovery.candidate_token())? != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_AUTHORITY_CHANGED");
    }
    Ok(())
}

pub(in crate::node_agent_compute_plugin_host::local_authority) fn read_exact_sealed_plan(
    transaction: &Transaction<'_>,
    expected: &HashedComputePluginCandidateCleanupExecutionPlan,
    candidate_token: &str,
) -> Result<Option<HashedComputePluginCandidateCleanupExecutionPlan>> {
    let expected_plan = expected.plan();
    let row = transaction
        .query_row(
            r#"SELECT candidate_token, authorization_receipt_digest,
                      installation_id_digest, root_identity_digest,
                      candidate_parent_anchor_relative_path,
                      candidate_parent_anchor_identity_digest,
                      object_count, file_count, directory_count, expected_file_bytes,
                      process_owner_epoch, planned_at_ms, plan_json, plan_digest
               FROM candidate_cleanup_execution_plans WHERE cleanup_id = ?1"#,
            params![expected_plan.cleanup_id()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_PLAN_READ")?;
    let Some(row) = row else { return Ok(None) };
    let plan: ComputePluginCandidateCleanupExecutionPlan = serde_json::from_str(&row.12)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_PLAN_DECODE")?;
    if row.0 != candidate_token
        || row.1 != plan.authorization_receipt_digest()
        || row.2 != plan.installation_id_digest()
        || row.3 != plan.root_identity_digest()
        || row.4 != plan.candidate_parent_anchor_relative_path()
        || row.5 != plan.candidate_parent_anchor_identity_digest()
        || row.6 != plan.object_count()
        || row.7 != plan.file_count()
        || row.8 != plan.directory_count()
        || row.9 != plan.expected_file_bytes()
        || row.10 != plan.process_owner_epoch()
        || row.11 != plan.planned_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_PLAN_ROW_CHANGED");
    }
    let objects = read_objects(transaction, plan.cleanup_id())?;
    let seal_count = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_execution_plan_seals
           WHERE cleanup_id = ?1 AND candidate_token = ?2 AND plan_digest = ?3
             AND object_count = ?4 AND sealed_at_ms = ?5"#,
            params![
                plan.cleanup_id(),
                candidate_token,
                row.13,
                plan.object_count(),
                plan.planned_at_ms()
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_SEAL_READ")?;
    if seal_count != 1 || i64::try_from(objects.len()).ok() != Some(plan.object_count()) {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_SEAL_CHANGED");
    }
    let restored = restore_hashed_execution_plan(plan, objects, row.13)?;
    if &restored != expected {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_READBACK_CHANGED");
    }
    Ok(Some(restored))
}

fn read_objects(
    transaction: &Transaction<'_>,
    cleanup_id: &str,
) -> Result<Vec<HashedCandidateCleanupExpectedObject>> {
    let mut statement = transaction
        .prepare(
            r#"SELECT step_ordinal, parent_step_ordinal, topology_depth, object_kind,
                  relative_name, relative_path, relative_path_digest,
                  expected_identity_digest, expected_parent_identity_digest,
                  expected_content_digest, expected_size_bytes, object_json, object_digest
           FROM candidate_cleanup_expected_objects
           WHERE cleanup_id = ?1 ORDER BY step_ordinal"#,
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_PREPARE")?;
    let rows = statement
        .query_map(params![cleanup_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
            ))
        })
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_QUERY")?;
    let mut objects = Vec::new();
    for row in rows {
        let row = row.context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_ROW")?;
        let object: ComputePluginCandidateCleanupExpectedObject = serde_json::from_str(&row.11)
            .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_DECODE")?;
        if row.0 != object.step_ordinal()
            || row.1 != object.parent_step_ordinal()
            || row.2 != object.topology_depth()
            || row.3 != object.object_kind()
            || row.4 != object.relative_name()
            || row.5 != object.relative_path()
            || row.6 != object.relative_path_digest()
            || row.7 != object.expected_identity_digest()
            || row.8 != object.expected_parent_identity_digest()
            || row.9.as_deref() != object.expected_content_digest()
            || row.10 != object.expected_size_bytes()
            || object.cleanup_id() != cleanup_id
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_ROW_CHANGED");
        }
        objects.push(restore_hashed_expected_object(object, row.12)?);
    }
    Ok(objects)
}

pub(super) fn count_plan_identity_matches(
    transaction: &Transaction<'_>,
    candidate_token: &str,
    plan: &HashedComputePluginCandidateCleanupExecutionPlan,
) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_execution_plans
           WHERE cleanup_id = ?1 OR candidate_token = ?2
              OR authorization_receipt_digest = ?3 OR plan_digest = ?4"#,
            params![
                plan.plan().cleanup_id(),
                candidate_token,
                plan.plan().authorization_receipt_digest(),
                plan.plan_digest()
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_IDENTITY_READ")
}

pub(super) fn count_objects(transaction: &Transaction<'_>, cleanup_id: &str) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_cleanup_expected_objects WHERE cleanup_id = ?1",
            params![cleanup_id],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_COUNT")
}

pub(super) fn count_seals(transaction: &Transaction<'_>, cleanup_id: &str) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_cleanup_execution_plan_seals WHERE cleanup_id = ?1",
            params![cleanup_id],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_SEAL_COUNT")
}

fn count_exact_authorization(
    transaction: &Transaction<'_>,
    state: &CandidateCleanupExecutionState,
) -> Result<i64> {
    let authorization = state.authorization_receipt();
    let receipt = authorization.receipt();
    let receipt_json = serde_json::to_string(receipt)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_AUTHORIZATION_SERIALIZE")?;
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_cleanup_authorizations
           WHERE cleanup_id = ?1 AND candidate_token = ?2
             AND candidate_token_digest = ?3 AND receipt_json = ?4 AND receipt_digest = ?5
             AND process_owner_epoch = ?6 AND authorized_at_ms = ?7
             AND slot_phase_before = 'failed'"#,
            params![
                receipt.cleanup_id(),
                state.staging_recovery_key().candidate_token(),
                receipt.candidate_token_digest(),
                receipt_json,
                authorization.receipt_digest(),
                receipt.process_owner_epoch(),
                receipt.authorized_at_ms()
            ],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_AUTHORIZATION_READ")
}

fn count_pending_owner(transaction: &Transaction<'_>, candidate_token: &str) -> Result<i64> {
    transaction
        .query_row(
            r#"SELECT COUNT(*) FROM candidate_owners
           WHERE candidate_token = ?1 AND state = 'cleanup_pending'
             AND closed_at_ms IS NULL AND closed_by_plan_id IS NULL
             AND closed_by_plan_digest IS NULL AND close_reason IS NULL"#,
            params![candidate_token],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OWNER_READ")
}

fn count_completion(transaction: &Transaction<'_>, candidate_token: &str) -> Result<i64> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM candidate_cleanup_completions WHERE candidate_token = ?1",
            params![candidate_token],
            |row| row.get(0),
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_COMPLETION_READ")
}
