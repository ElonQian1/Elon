use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

use super::{
    validation::{read_exact_sealed_plan, validate_authority_and_owner, validate_unsealed_binding},
    ComputePluginCandidateCleanupTopologyAuthoritySession,
};
use crate::node_agent_compute_plugin_host::{
    candidate_cleanup_contract::{
        HashedComputePluginCandidateCleanupExecutionPlan, ValidatedCandidateCleanupTopologyPermit,
    },
    local_authority::keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
};

pub(super) fn persist_candidate_cleanup_topology(
    transaction: &Transaction<'_>,
    session: &ComputePluginCandidateCleanupTopologyAuthoritySession<'_>,
    permit: ValidatedCandidateCleanupTopologyPermit<'_>,
) -> Result<HashedComputePluginCandidateCleanupExecutionPlan> {
    let state = permit.state();
    let plan = permit.plan();
    session.validate_source(state.cancellation_guard())?;
    validate_unsealed_binding(transaction, session, state, plan)?;
    let authorization = state.authorization_receipt().receipt();
    let time_state = read_authority_keyring_state(transaction)?;
    if time_state.state_revision != authorization.authority_state_revision_after()
        || time_state.authority_epoch != authorization.authority_epoch_after()
        || time_state.trusted_time_high_water_ms != Some(authorization.authorized_at_ms())
        || time_state.clock_status != "trusted"
        || plan.plan().planned_at_ms() <= authorization.authorized_at_ms()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_TIME_CHANGED");
    }
    advance_trusted_time(transaction, &time_state, plan.plan().planned_at_ms())?;
    session.validate_source(state.cancellation_guard())?;

    insert_plan(
        transaction,
        state.staging_recovery_key().candidate_token(),
        plan,
    )?;
    insert_objects(transaction, plan)?;
    insert_seal(
        transaction,
        state.staging_recovery_key().candidate_token(),
        plan,
    )?;
    validate_authority_and_owner(transaction, session, state, plan.plan().planned_at_ms())?;
    let stored = read_exact_sealed_plan(
        transaction,
        plan,
        state.staging_recovery_key().candidate_token(),
    )?
    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_READBACK_MISSING"))?;
    session.validate_source(state.cancellation_guard())?;
    Ok(stored)
}

fn insert_plan(
    transaction: &Transaction<'_>,
    candidate_token: &str,
    hashed: &HashedComputePluginCandidateCleanupExecutionPlan,
) -> Result<()> {
    let plan = hashed.plan();
    let plan_json = serde_json::to_string(plan)
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_PLAN_SERIALIZE")?;
    transaction
        .execute(
            r#"INSERT INTO candidate_cleanup_execution_plans (
               cleanup_id, candidate_token, authorization_receipt_digest,
               installation_id_digest, root_identity_digest,
               candidate_parent_anchor_relative_path,
               candidate_parent_anchor_identity_digest,
               object_count, file_count, directory_count, expected_file_bytes,
               process_owner_epoch, planned_at_ms, plan_json, plan_digest
           ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"#,
            params![
                plan.cleanup_id(),
                candidate_token,
                plan.authorization_receipt_digest(),
                plan.installation_id_digest(),
                plan.root_identity_digest(),
                plan.candidate_parent_anchor_relative_path(),
                plan.candidate_parent_anchor_identity_digest(),
                plan.object_count(),
                plan.file_count(),
                plan.directory_count(),
                plan.expected_file_bytes(),
                plan.process_owner_epoch(),
                plan.planned_at_ms(),
                plan_json,
                hashed.plan_digest(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_PLAN_INSERT")?;
    Ok(())
}

fn insert_objects(
    transaction: &Transaction<'_>,
    hashed: &HashedComputePluginCandidateCleanupExecutionPlan,
) -> Result<()> {
    for expected in hashed.objects() {
        let object = expected.object();
        let object_json = serde_json::to_string(object)
            .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_SERIALIZE")?;
        transaction
            .execute(
                r#"INSERT INTO candidate_cleanup_expected_objects (
                   cleanup_id, step_ordinal, parent_step_ordinal, topology_depth,
                   object_kind, relative_name, relative_path, relative_path_digest,
                   expected_identity_digest, expected_parent_identity_digest,
                   expected_content_digest, expected_size_bytes, object_json, object_digest
               ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
                params![
                    object.cleanup_id(),
                    object.step_ordinal(),
                    object.parent_step_ordinal(),
                    object.topology_depth(),
                    object.object_kind(),
                    object.relative_name(),
                    object.relative_path(),
                    object.relative_path_digest(),
                    object.expected_identity_digest(),
                    object.expected_parent_identity_digest(),
                    object.expected_content_digest(),
                    object.expected_size_bytes(),
                    object_json,
                    expected.object_digest(),
                ],
            )
            .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_OBJECT_INSERT")?;
    }
    Ok(())
}

fn insert_seal(
    transaction: &Transaction<'_>,
    candidate_token: &str,
    hashed: &HashedComputePluginCandidateCleanupExecutionPlan,
) -> Result<()> {
    let plan = hashed.plan();
    transaction
        .execute(
            r#"INSERT INTO candidate_cleanup_execution_plan_seals (
               cleanup_id, candidate_token, plan_digest, object_count, sealed_at_ms
           ) VALUES (?1, ?2, ?3, ?4, ?5)"#,
            params![
                plan.cleanup_id(),
                candidate_token,
                hashed.plan_digest(),
                plan.object_count(),
                plan.planned_at_ms(),
            ],
        )
        .context("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_TOPOLOGY_SEAL_INSERT")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::*;
    use crate::node_agent_compute_plugin_host::{
        candidate_cleanup_contract::{
            restore_hashed_execution_plan, restore_hashed_expected_object,
            ComputePluginCandidateCleanupExecutionPlan,
            ComputePluginCandidateCleanupExpectedObject,
        },
        signed_artifact_verification::jcs_sha256_hex,
    };

    const CANDIDATE_TOKEN: &str = "candidate-token";

    fn execution_plan() -> HashedComputePluginCandidateCleanupExecutionPlan {
        let object: ComputePluginCandidateCleanupExpectedObject = serde_json::from_value(
            serde_json::json!({
                "schema": "elon.compute_plugin.candidate_cleanup_expected_object.v1",
                "cleanup_id": "cca_store_round_trip",
                "step_ordinal": 0,
                "parent_step_ordinal": null,
                "topology_depth": 0,
                "object_kind": "directory",
                "logical_kind": "candidate_directory",
                "relative_name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "relative_path": "compute-plugin/candidates/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "relative_path_digest": "1".repeat(64),
                "expected_identity_digest": "2".repeat(64),
                "expected_parent_identity_digest": "3".repeat(64),
                "expected_content_digest": null,
                "expected_size_bytes": null
            }),
        )
        .unwrap();
        let object_digest = jcs_sha256_hex(&object).unwrap();
        let object = restore_hashed_expected_object(object, object_digest.clone()).unwrap();
        let plan: ComputePluginCandidateCleanupExecutionPlan =
            serde_json::from_value(serde_json::json!({
                "schema": "elon.compute_plugin.candidate_cleanup_execution_plan.v1",
                "cleanup_id": "cca_store_round_trip",
                "candidate_token_digest": "a".repeat(64),
                "authorization_receipt_digest": "4".repeat(64),
                "installation_id_digest": "5".repeat(64),
                "root_identity_digest": "6".repeat(64),
                "candidate_parent_anchor_relative_path": "compute-plugin/candidates",
                "candidate_parent_anchor_identity_digest": "3".repeat(64),
                "object_count": 1,
                "file_count": 0,
                "directory_count": 1,
                "expected_file_bytes": 0,
                "process_owner_epoch": 7,
                "planned_at_ms": 2_000,
                "object_digests": [object_digest]
            }))
            .unwrap();
        let plan_digest = jcs_sha256_hex(&plan).unwrap();
        restore_hashed_execution_plan(plan, vec![object], plan_digest).unwrap()
    }

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE candidate_cleanup_execution_plans (
                    cleanup_id TEXT PRIMARY KEY, candidate_token TEXT,
                    authorization_receipt_digest TEXT, installation_id_digest TEXT,
                    root_identity_digest TEXT, candidate_parent_anchor_relative_path TEXT,
                    candidate_parent_anchor_identity_digest TEXT, object_count INTEGER,
                    file_count INTEGER, directory_count INTEGER, expected_file_bytes INTEGER,
                    process_owner_epoch INTEGER, planned_at_ms INTEGER,
                    plan_json TEXT, plan_digest TEXT
                );
                CREATE TABLE candidate_cleanup_expected_objects (
                    cleanup_id TEXT, step_ordinal INTEGER, parent_step_ordinal INTEGER,
                    topology_depth INTEGER, object_kind TEXT, relative_name TEXT,
                    relative_path TEXT, relative_path_digest TEXT,
                    expected_identity_digest TEXT, expected_parent_identity_digest TEXT,
                    expected_content_digest TEXT, expected_size_bytes INTEGER,
                    object_json TEXT, object_digest TEXT
                );
                CREATE TABLE candidate_cleanup_execution_plan_seals (
                    cleanup_id TEXT, candidate_token TEXT, plan_digest TEXT,
                    object_count INTEGER, sealed_at_ms INTEGER
                );
                "#,
            )
            .unwrap();
        connection
    }

    fn write_plan(
        transaction: &Transaction<'_>,
        plan: &HashedComputePluginCandidateCleanupExecutionPlan,
    ) {
        insert_plan(transaction, CANDIDATE_TOKEN, plan).unwrap();
        insert_objects(transaction, plan).unwrap();
        insert_seal(transaction, CANDIDATE_TOKEN, plan).unwrap();
    }

    #[test]
    fn cleanup_topology_store_round_trips_exact_plan() {
        let mut connection = connection();
        let transaction = connection.transaction().unwrap();
        let plan = execution_plan();
        write_plan(&transaction, &plan);

        let stored = read_exact_sealed_plan(&transaction, &plan, CANDIDATE_TOKEN)
            .unwrap()
            .unwrap();

        assert_eq!(stored, plan);
    }

    #[test]
    fn cleanup_topology_store_rejects_changed_object_column() {
        let mut connection = connection();
        let transaction = connection.transaction().unwrap();
        let plan = execution_plan();
        write_plan(&transaction, &plan);
        transaction
            .execute(
                "UPDATE candidate_cleanup_expected_objects SET relative_path = ?1",
                params!["compute-plugin/candidates/changed"],
            )
            .unwrap();

        let error = read_exact_sealed_plan(&transaction, &plan, CANDIDATE_TOKEN).unwrap_err();

        assert!(error.to_string().contains("OBJECT_ROW_CHANGED"));
    }
}
