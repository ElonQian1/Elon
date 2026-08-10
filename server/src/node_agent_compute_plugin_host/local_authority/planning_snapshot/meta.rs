use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Transaction;
use serde::Serialize;

use super::super::plan_application::{
    read_authority_plan_application_state, AuthorityPlanApplicationState,
};
use crate::node_agent_compute_plugin_host::{
    local_authority_schema::{
        verify_schema_v8_read_only, COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

const MAX_IJSON_INTEGER: i64 = 9_007_199_254_740_991;

pub(super) struct PlanningAuthorityRead {
    pub(super) state: AuthorityPlanApplicationState,
    pub(super) schema_version: i64,
    pub(super) updated_at_ms: i64,
    pub(super) captured_at_ms: u64,
    fingerprint: String,
}

#[derive(Serialize)]
struct AuthorityFingerprint<'a> {
    schema_version: i64,
    installation_id_digest: &'a str,
    state_revision: i64,
    authority_epoch: i64,
    process_owner_epoch: i64,
    inventory_json: &'a str,
    inventory_digest: &'a str,
    desired_policy_revision: i64,
    sharing_enabled: bool,
    sharing_authorization: &'a Option<
        crate::node_agent_compute_plugin_host::install_plan::ComputeSharingAuthorizationBinding,
    >,
    node_profile_digest: &'a str,
    manifest_catalog_revision: i64,
    target_id: &'a str,
    host_api_protocol_id: &'a str,
    host_api_revision: u32,
    keyring_bundle_revision: i64,
    publisher_keyring_revision: i64,
    publisher_keyring_digest: &'a str,
    control_keyring_revision: i64,
    control_keyring_digest: &'a str,
    trusted_time_high_water_ms: i64,
    updated_at_ms: i64,
}

pub(super) fn read_planning_authority_on(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<PlanningAuthorityRead> {
    verify_schema_v8_read_only(transaction)?;
    let state = read_authority_plan_application_state(transaction, trusted_now)?;
    let (schema_version, updated_at_ms) = transaction
        .query_row(
            "SELECT schema_version, updated_at_ms FROM authority_meta WHERE singleton = 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .context("COMPUTE_PLUGIN_PLANNING_AUTHORITY_UPDATED_AT_READ")?;
    validate_state(&state, schema_version, updated_at_ms, trusted_now)?;
    let captured_at_ms = u64::try_from(trusted_now.timestamp_millis())
        .context("COMPUTE_PLUGIN_PLANNING_AUTHORITY_CAPTURE_TIME")?;
    let fingerprint = fingerprint(&state, schema_version, updated_at_ms)?;
    Ok(PlanningAuthorityRead {
        state,
        schema_version,
        updated_at_ms,
        captured_at_ms,
        fingerprint,
    })
}

pub(super) fn ensure_planning_authority_unchanged_on(
    transaction: &Transaction<'_>,
    trusted_now: &DateTime<Utc>,
    expected: &PlanningAuthorityRead,
) -> Result<()> {
    let actual = read_planning_authority_on(transaction, trusted_now)?;
    if actual.fingerprint != expected.fingerprint {
        bail!("COMPUTE_PLUGIN_PLANNING_AUTHORITY_CHANGED");
    }
    Ok(())
}

fn validate_state(
    state: &AuthorityPlanApplicationState,
    schema_version: i64,
    updated_at_ms: i64,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    let captured_at_ms = trusted_now.timestamp_millis();
    let integer_facts = [
        state.state_revision,
        state.authority_epoch,
        state.process_owner_epoch,
        state.inventory.inventory_revision,
        state.desired_policy_revision,
        state.manifest_catalog_revision,
        state.keyring_bundle_revision,
        state.publisher_keyring.revision,
        state.control_keyring.revision,
        state.trusted_time_high_water_ms,
        updated_at_ms,
        captured_at_ms,
    ];
    if schema_version != COMPUTE_PLUGIN_LOCAL_AUTHORITY_SCHEMA_VERSION
        || integer_facts
            .into_iter()
            .any(|value| !(0..=MAX_IJSON_INTEGER).contains(&value))
        || state.state_revision == 0
        || state.authority_epoch == 0
        || state.process_owner_epoch == 0
        || state.desired_policy_revision == 0
        || state.manifest_catalog_revision == 0
        || state.keyring_bundle_revision == 0
        || state.publisher_keyring.revision == 0
        || state.control_keyring.revision == 0
        || state.trusted_time_high_water_ms == 0
        || updated_at_ms != state.trusted_time_high_water_ms
        || captured_at_ms <= state.trusted_time_high_water_ms
        || !is_sha256(&state.installation_id_digest)
        || !is_sha256(&state.inventory_digest)
        || !is_sha256(&state.node_profile_digest)
        || !is_sha256(&state.publisher_keyring.digest)
        || !is_sha256(&state.control_keyring.digest)
        || state.publisher_keyring == state.control_keyring
    {
        bail!("COMPUTE_PLUGIN_PLANNING_AUTHORITY_META_INVALID");
    }
    Ok(())
}

fn fingerprint(
    state: &AuthorityPlanApplicationState,
    schema_version: i64,
    updated_at_ms: i64,
) -> Result<String> {
    jcs_sha256_hex(&AuthorityFingerprint {
        schema_version,
        installation_id_digest: &state.installation_id_digest,
        state_revision: state.state_revision,
        authority_epoch: state.authority_epoch,
        process_owner_epoch: state.process_owner_epoch,
        inventory_json: &state.inventory_json,
        inventory_digest: &state.inventory_digest,
        desired_policy_revision: state.desired_policy_revision,
        sharing_enabled: state.sharing_enabled,
        sharing_authorization: &state.sharing_authorization,
        node_profile_digest: &state.node_profile_digest,
        manifest_catalog_revision: state.manifest_catalog_revision,
        target_id: &state.target_id,
        host_api_protocol_id: &state.host_api_protocol_id,
        host_api_revision: state.host_api_revision,
        keyring_bundle_revision: state.keyring_bundle_revision,
        publisher_keyring_revision: state.publisher_keyring.revision,
        publisher_keyring_digest: &state.publisher_keyring.digest,
        control_keyring_revision: state.control_keyring.revision,
        control_keyring_digest: &state.control_keyring.digest,
        trusted_time_high_water_ms: state.trusted_time_high_water_ms,
        updated_at_ms,
    })
}
