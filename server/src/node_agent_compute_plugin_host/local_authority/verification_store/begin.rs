use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{named_params, OptionalExtension, Transaction};

use super::{
    closure, read, ComputePluginCandidateVerificationAuthorityFacts,
    ComputePluginPreparedCandidateVerificationFacts,
};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::ValidatedCandidateVerificationBeginPermit,
    install_plan_admission_validation::is_identifier,
    keyring::ComputePluginBootstrapRootKeyResolver, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::{
    keyring_snapshot::{advance_trusted_time, read_authority_keyring_state},
    ComputePluginFetchProcessFence,
};

#[derive(Clone, PartialEq, Eq)]
struct BeginAuthorityRow {
    installation_id_digest: String,
    state_revision: i64,
    inventory_revision: i64,
    inventory_digest: String,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: i64,
    clock_status: String,
    sharing_enabled: i64,
    updated_at_ms: i64,
}

pub(super) fn begin_candidate_verification(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
    permit: ValidatedCandidateVerificationBeginPermit<'_>,
) -> Result<ComputePluginPreparedCandidateVerificationFacts> {
    validate_permit(&permit, &trusted_now)?;
    let key = permit.key();
    let current = read::read_fresh_candidate_verification_authority(
        transaction,
        process_fence,
        trusted_now.clone(),
        roots,
        key.owner_plan_id(),
        key.owner_plan_digest(),
        key.candidate_token(),
    )?;
    if &current != permit.authority() {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_AUTHORITY_CAS");
    }
    validate_current(key, &current)?;
    let closure_before =
        closure::read_candidate_closure_snapshot(transaction, key.candidate_token())?;
    if closure_before.durable_closure_digest != key.durable_candidate_closure_digest()
        || closure_before.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || closure_before.artifact_count != key.artifact_count()
        || closure_before.artifact_bytes != key.artifact_bytes()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_CLOSURE_CAS");
    }
    let authority_before = read_begin_authority(transaction)?;
    validate_authority_before(&authority_before, key, &current)?;
    let state = read_authority_keyring_state(transaction)?;
    if state.state_revision != key.authority_state_revision()
        || state.authority_epoch != key.authority_epoch()
        || state.trusted_time_high_water_ms != Some(current.observed_trusted_time_high_water_ms)
        || state.clock_status != "trusted"
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_TIME_FENCE_CHANGED");
    }
    advance_trusted_time(transaction, &state, key.prepared_at_ms())?;
    insert_prepared_run(transaction, key)?;
    let prepared = read_exact_prepared_run(transaction, key)?;
    let authority_after = read_begin_authority(transaction)?;
    let mut expected_authority = authority_before;
    expected_authority.trusted_time_high_water_ms = key.prepared_at_ms();
    expected_authority.clock_status = "trusted".to_string();
    expected_authority.updated_at_ms = key.prepared_at_ms();
    if authority_after != expected_authority {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_AUTHORITY_POST_WRITE_MISMATCH");
    }
    let closure_after =
        closure::read_candidate_closure_snapshot(transaction, key.candidate_token())?;
    if closure_after.durable_closure_digest != closure_before.durable_closure_digest
        || closure_after.expected_artifact_set_digest != closure_before.expected_artifact_set_digest
        || closure_after.artifact_count != closure_before.artifact_count
        || closure_after.artifact_bytes != closure_before.artifact_bytes
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_CLOSURE_POST_WRITE_MISMATCH");
    }
    Ok(prepared)
}

fn validate_permit(
    permit: &ValidatedCandidateVerificationBeginPermit<'_>,
    trusted_now: &DateTime<Utc>,
) -> Result<()> {
    let key = permit.key();
    if !is_identifier(key.verification_id())
        || !is_identifier(key.candidate_token())
        || !is_identifier(key.owner_plan_id())
        || !is_sha256(key.installation_id_digest())
        || !is_sha256(key.clock_epoch_digest())
        || !is_sha256(key.root_identity_digest())
        || !is_sha256(key.candidate_token_digest())
        || !is_sha256(key.owner_plan_digest())
        || !is_sha256(key.inventory_digest())
        || !is_sha256(key.expected_artifact_set_digest())
        || !is_sha256(key.durable_candidate_closure_digest())
        || !is_sha256(key.file_set_binding_digest())
        || key.verification_generation() <= 0
        || key.candidate_generation() <= 0
        || key.application_inventory_revision() <= 0
        || key.authority_state_revision() <= 0
        || key.authority_epoch() <= 0
        || key.process_owner_epoch() <= 0
        || key.artifact_count() == 0
        || key.artifact_count() > 4_096
        || key.artifact_bytes() <= 0
        || key.prepared_at_ms() != trusted_now.timestamp_millis()
        || jcs_sha256_hex(&key.candidate_token())? != key.candidate_token_digest()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_PERMIT_INVALID");
    }
    Ok(())
}

fn validate_current(
    key: &crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    current: &ComputePluginCandidateVerificationAuthorityFacts,
) -> Result<()> {
    if current.installation_id_digest != key.installation_id_digest()
        || current.applied_plan_id != key.owner_plan_id()
        || current.applied_plan_digest != key.owner_plan_digest()
        || current.candidate_owner_plan_id != key.owner_plan_id()
        || current.candidate_owner_plan_digest != key.owner_plan_digest()
        || current.next_verification_generation != key.verification_generation()
        || current.candidate_generation != key.candidate_generation()
        || current.candidate_application_inventory_revision != key.application_inventory_revision()
        || current.authority_state_revision != key.authority_state_revision()
        || current.authority_epoch != key.authority_epoch()
        || current.process_owner_epoch != key.process_owner_epoch()
        || current.execution_inventory_revision != key.execution_inventory_revision()
        || current.inventory_digest != key.inventory_digest()
        || current.candidate_token_digest != key.candidate_token_digest()
        || current.artifacts.len() != key.artifact_count()
        || current.artifact_bytes != key.artifact_bytes()
        || current.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || current.trusted_now.timestamp_millis() != key.prepared_at_ms()
        || key.prepared_at_ms() <= current.observed_trusted_time_high_water_ms
        || current.recompute_durable_candidate_closure_digest()?
            != key.durable_candidate_closure_digest()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_CURRENT_CHANGED");
    }
    Ok(())
}

fn read_begin_authority(transaction: &Transaction<'_>) -> Result<BeginAuthorityRow> {
    transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, inventory_revision,
                inventory_digest, authority_epoch, process_owner_epoch,
                trusted_time_high_water_ms, clock_status, sharing_enabled, updated_at_ms
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok(BeginAuthorityRow {
                    installation_id_digest: row.get(0)?,
                    state_revision: row.get(1)?,
                    inventory_revision: row.get(2)?,
                    inventory_digest: row.get(3)?,
                    authority_epoch: row.get(4)?,
                    process_owner_epoch: row.get(5)?,
                    trusted_time_high_water_ms: row.get::<_, Option<i64>>(6)?.unwrap_or(-1),
                    clock_status: row.get(7)?,
                    sharing_enabled: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_VERIFICATION_BEGIN_AUTHORITY_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))
}

fn validate_authority_before(
    authority: &BeginAuthorityRow,
    key: &crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
    current: &ComputePluginCandidateVerificationAuthorityFacts,
) -> Result<()> {
    if authority.installation_id_digest != key.installation_id_digest()
        || authority.state_revision != key.authority_state_revision()
        || authority.inventory_revision != key.execution_inventory_revision()
        || authority.inventory_digest != key.inventory_digest()
        || authority.authority_epoch != key.authority_epoch()
        || authority.process_owner_epoch != key.process_owner_epoch()
        || authority.trusted_time_high_water_ms != current.observed_trusted_time_high_water_ms
        || authority.clock_status != "trusted"
        || authority.sharing_enabled != 1
        || authority.updated_at_ms < 0
        || authority.updated_at_ms > authority.trusted_time_high_water_ms
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_AUTHORITY_CHANGED");
    }
    Ok(())
}

fn insert_prepared_run(
    transaction: &Transaction<'_>,
    key: &crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
) -> Result<()> {
    let inserted = transaction
        .execute(
            r#"INSERT INTO candidate_verification_runs (
                verification_id, candidate_token, owner_plan_id, owner_plan_digest,
                verification_generation, candidate_generation,
                application_inventory_revision, authority_state_revision,
                authority_epoch, process_owner_epoch, artifact_count, artifact_bytes,
                expected_artifact_set_digest, file_set_binding_digest,
                state, prepared_at_ms, resolved_at_ms, resolution_reason,
                result_json, result_digest, mismatch_ordinal, observed_digest
            ) VALUES (
                :verification_id, :candidate_token, :owner_plan_id, :owner_plan_digest,
                :verification_generation, :candidate_generation,
                :application_inventory_revision, :authority_state_revision,
                :authority_epoch, :process_owner_epoch, :artifact_count, :artifact_bytes,
                :expected_artifact_set_digest, :file_set_binding_digest,
                'prepared', :prepared_at_ms, NULL, NULL, NULL, NULL, NULL, NULL
            )"#,
            named_params! {
                ":verification_id": key.verification_id(),
                ":candidate_token": key.candidate_token(),
                ":owner_plan_id": key.owner_plan_id(),
                ":owner_plan_digest": key.owner_plan_digest(),
                ":verification_generation": key.verification_generation(),
                ":candidate_generation": key.candidate_generation(),
                ":application_inventory_revision": key.application_inventory_revision(),
                ":authority_state_revision": key.authority_state_revision(),
                ":authority_epoch": key.authority_epoch(),
                ":process_owner_epoch": key.process_owner_epoch(),
                ":artifact_count": i64::try_from(key.artifact_count())
                    .context("COMPUTE_PLUGIN_VERIFICATION_BEGIN_ARTIFACT_COUNT")?,
                ":artifact_bytes": key.artifact_bytes(),
                ":expected_artifact_set_digest": key.expected_artifact_set_digest(),
                ":file_set_binding_digest": key.file_set_binding_digest(),
                ":prepared_at_ms": key.prepared_at_ms(),
            },
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_BEGIN_INSERT")?;
    if inserted != 1 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_INSERT_CAS");
    }
    Ok(())
}

pub(super) fn read_exact_prepared_run(
    transaction: &Transaction<'_>,
    key: &crate::node_agent_compute_plugin_host::candidate_verification_contract::ComputePluginCandidateVerificationRecoveryKey,
) -> Result<ComputePluginPreparedCandidateVerificationFacts> {
    type Row = (
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        String,
        String,
        String,
        i64,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
    );
    let row: Row = transaction
        .query_row(
            r#"SELECT verification_id, candidate_token, owner_plan_id, owner_plan_digest,
                verification_generation, candidate_generation, application_inventory_revision,
                authority_state_revision, authority_epoch, process_owner_epoch,
                artifact_count, artifact_bytes, expected_artifact_set_digest,
                file_set_binding_digest, state, prepared_at_ms, resolved_at_ms,
                resolution_reason, result_json, result_digest, mismatch_ordinal, observed_digest
            FROM candidate_verification_runs WHERE verification_id = ?1"#,
            [key.verification_id()],
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
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                ))
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_VERIFICATION_BEGIN_READ_BACK")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_ROW_MISSING"))?;
    let artifact_count =
        usize::try_from(row.10).context("COMPUTE_PLUGIN_VERIFICATION_BEGIN_ARTIFACT_COUNT")?;
    if row.0 != key.verification_id()
        || row.1 != key.candidate_token()
        || row.2 != key.owner_plan_id()
        || row.3 != key.owner_plan_digest()
        || row.4 != key.verification_generation()
        || row.5 != key.candidate_generation()
        || row.6 != key.application_inventory_revision()
        || row.7 != key.authority_state_revision()
        || row.8 != key.authority_epoch()
        || row.9 != key.process_owner_epoch()
        || artifact_count != key.artifact_count()
        || row.11 != key.artifact_bytes()
        || row.12 != key.expected_artifact_set_digest()
        || row.13 != key.file_set_binding_digest()
        || row.14 != "prepared"
        || row.15 != key.prepared_at_ms()
        || row.16.is_some()
        || row.17.is_some()
        || row.18.is_some()
        || row.19.is_some()
        || row.20.is_some()
        || row.21.is_some()
        || jcs_sha256_hex(&row.1)? != key.candidate_token_digest()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_BEGIN_READ_BACK_CHANGED");
    }
    Ok(ComputePluginPreparedCandidateVerificationFacts {
        verification_id: row.0,
        candidate_token_digest: key.candidate_token_digest().to_string(),
        owner_plan_id: row.2,
        owner_plan_digest: row.3,
        verification_generation: row.4,
        candidate_generation: row.5,
        application_inventory_revision: row.6,
        authority_state_revision: row.7,
        authority_epoch: row.8,
        process_owner_epoch: row.9,
        artifact_count,
        artifact_bytes: row.11,
        expected_artifact_set_digest: row.12,
        file_set_binding_digest: row.13,
        prepared_at_ms: row.15,
    })
}
