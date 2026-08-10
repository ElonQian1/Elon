use anyhow::Result;
use rusqlite::Connection;

use super::super::{
    authority_fences, candidate_cleanup, candidate_cleanup_execution, candidate_cleanup_journal,
    candidate_health, candidate_health_quarantine, candidate_promotion, candidate_staging,
    candidate_verification, fetch_claims, manifest_catalog_binding, plan_application,
    schema_integrity, sharing_policy_binding, sharing_policy_revocation, work_admission, SCHEMA_V3,
};

const BASE_SCHEMA_DEFINITION_BATCHES_V8: [&str; 14] = [
    SCHEMA_V3,
    candidate_verification::CANDIDATE_VERIFICATION_SCHEMA_V3,
    candidate_staging::CANDIDATE_STAGING_SCHEMA_V3,
    candidate_health::CANDIDATE_HEALTH_SCHEMA_V3,
    candidate_health_quarantine::CANDIDATE_HEALTH_QUARANTINE_SCHEMA_V3,
    candidate_cleanup::CANDIDATE_CLEANUP_SCHEMA_V3,
    candidate_cleanup_execution::CANDIDATE_CLEANUP_EXECUTION_SCHEMA_V3,
    candidate_cleanup_journal::CANDIDATE_CLEANUP_JOURNAL_SCHEMA_V3,
    plan_application::PLAN_APPLICATION_SCHEMA_V3,
    authority_fences::AUTHORITY_FENCE_SCHEMA_V3,
    fetch_claims::FETCH_CLAIM_SCHEMA_V3,
    sharing_policy_binding::SHARING_POLICY_BINDING_SCHEMA_V4,
    sharing_policy_revocation::SHARING_POLICY_REVOCATION_SCHEMA_V5,
    manifest_catalog_binding::MANIFEST_CATALOG_BINDING_SCHEMA_V6,
];

/// Source-frozen V8 manifest for planning. This path tokenizes DDL constants but never executes
/// them, opens another SQLite connection, installs schema, or mutates the authority connection.
pub(super) fn verify_schema_objects_v8_read_only(connection: &Connection) -> Result<()> {
    schema_integrity::verify_schema_objects_from_definitions(
        connection,
        BASE_SCHEMA_DEFINITION_BATCHES_V8
            .into_iter()
            .chain(candidate_promotion::schema_definition_batches_v7())
            .chain(work_admission::schema_definition_batches_v8()),
    )
}
