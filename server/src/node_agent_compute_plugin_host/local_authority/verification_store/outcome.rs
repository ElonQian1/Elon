use anyhow::{bail, Context, Result};
use rusqlite::{OptionalExtension, Transaction};

use super::closure::{self, CandidateClosureSnapshot};
use crate::node_agent_compute_plugin_host::{
    candidate_verification_contract::{
        ComputePluginCandidateVerificationOutcome, ComputePluginCandidateVerificationOutcomeKind,
        ComputePluginCandidateVerificationRecoveryKey,
    },
    candidate_verification_terminal_result::{
        validate_candidate_verification_terminal_result, CandidateVerificationTerminalKind,
    },
    install_plan_admission_validation::is_identifier,
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

use super::super::ComputePluginFetchProcessFence;

mod absence;
mod revocation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerificationRecoveryAuthorityRow {
    pub installation_id_digest: String,
    pub state_revision: i64,
    pub inventory_revision: i64,
    pub inventory_digest: String,
    pub inventory_json: String,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub trusted_time_high_water_ms: i64,
    pub clock_status: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerificationRunRow {
    pub verification_id: String,
    pub candidate_token: String,
    pub owner_plan_id: String,
    pub owner_plan_digest: String,
    pub verification_generation: i64,
    pub candidate_generation: i64,
    pub application_inventory_revision: i64,
    pub authority_state_revision: i64,
    pub authority_epoch: i64,
    pub process_owner_epoch: i64,
    pub artifact_count: i64,
    pub artifact_bytes: i64,
    pub expected_artifact_set_digest: String,
    pub file_set_binding_digest: String,
    pub state: String,
    pub prepared_at_ms: i64,
    pub resolved_at_ms: Option<i64>,
    pub resolution_reason: Option<String>,
    pub result_json: Option<String>,
    pub result_digest: Option<String>,
    pub mismatch_ordinal: Option<i64>,
    pub observed_digest: Option<String>,
}

pub(super) struct VerificationOutcomeSnapshot {
    pub authority: VerificationRecoveryAuthorityRow,
    pub run: Option<VerificationRunRow>,
    pub closure: Option<CandidateClosureSnapshot>,
    pub outcome: ComputePluginCandidateVerificationOutcome,
}

pub(super) fn read_outcome_snapshot(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<VerificationOutcomeSnapshot> {
    validate_key(key)?;
    let authority = read_recovery_authority(transaction)?;
    validate_reader(&authority, process_fence, key)?;
    let exists = exact_verification_id_exists(transaction, key)?;
    if !exists {
        let closure = absence::validate_not_created(transaction, &authority, key)?;
        let outcome = ComputePluginCandidateVerificationOutcome::from_store(
            ComputePluginCandidateVerificationOutcomeKind::NotCreated,
            key,
            None,
            None,
            None,
        );
        return Ok(VerificationOutcomeSnapshot {
            authority,
            run: None,
            closure: Some(closure),
            outcome,
        });
    }
    let run = read_run(transaction, key)?;
    validate_run_identity(&run, key)?;
    let closure = (run.state == "prepared")
        .then(|| closure::read_candidate_closure_snapshot(transaction, key.candidate_token()))
        .transpose()?;
    let outcome = classify_run(transaction, &authority, &run, closure.as_ref(), key)?;
    Ok(VerificationOutcomeSnapshot {
        authority,
        run: Some(run),
        closure,
        outcome,
    })
}

fn validate_key(key: &ComputePluginCandidateVerificationRecoveryKey) -> Result<()> {
    if !is_sha256(key.installation_id_digest())
        || !is_sha256(key.clock_epoch_digest())
        || !is_sha256(key.root_identity_digest())
        || !is_identifier(key.verification_id())
        || !is_identifier(key.candidate_token())
        || !is_sha256(key.candidate_token_digest())
        || !is_identifier(key.owner_plan_id())
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
        || key.prepared_at_ms() < 0
        || jcs_sha256_hex(&key.candidate_token())? != key.candidate_token_digest()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_KEY_INVALID");
    }
    Ok(())
}

pub(super) fn read_recovery_authority(
    transaction: &Transaction<'_>,
) -> Result<VerificationRecoveryAuthorityRow> {
    transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, inventory_revision,
                inventory_digest, inventory_json, authority_epoch, process_owner_epoch,
                trusted_time_high_water_ms, clock_status, updated_at_ms
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok(VerificationRecoveryAuthorityRow {
                    installation_id_digest: row.get(0)?,
                    state_revision: row.get(1)?,
                    inventory_revision: row.get(2)?,
                    inventory_digest: row.get(3)?,
                    inventory_json: row.get(4)?,
                    authority_epoch: row.get(5)?,
                    process_owner_epoch: row.get(6)?,
                    trusted_time_high_water_ms: row.get::<_, Option<i64>>(7)?.unwrap_or(-1),
                    clock_status: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_AUTHORITY_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_UNINITIALIZED"))
}

fn validate_reader(
    authority: &VerificationRecoveryAuthorityRow,
    process_fence: &ComputePluginFetchProcessFence,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<()> {
    if !is_sha256(&authority.installation_id_digest)
        || authority.installation_id_digest != key.installation_id_digest()
        || authority.installation_id_digest != process_fence.installation_id_digest()
        || authority.state_revision < key.authority_state_revision()
        || authority.inventory_revision < key.execution_inventory_revision()
        || authority.authority_epoch < key.authority_epoch()
        || authority.process_owner_epoch < key.process_owner_epoch()
        || authority.process_owner_epoch != process_fence.process_owner_epoch()
        || process_fence.acquired_at_ms() < 0
        || process_fence.acquired_at_ms() > authority.trusted_time_high_water_ms
        || authority.trusted_time_high_water_ms < 0
        || authority.updated_at_ms < 0
        || authority.updated_at_ms > authority.trusted_time_high_water_ms
        || !matches!(
            authority.clock_status.as_str(),
            "trusted" | "clock_untrusted"
        )
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_READER_CHANGED");
    }
    Ok(())
}

pub(super) fn exact_verification_id_exists(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<bool> {
    let exists = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM candidate_verification_runs WHERE verification_id = ?1)",
            [key.verification_id()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_EXISTENCE_READ")?;
    match exists {
        0 => Ok(false),
        1 => Ok(true),
        _ => bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_EXISTENCE_CORRUPT"),
    }
}

fn read_run(
    transaction: &Transaction<'_>,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<VerificationRunRow> {
    transaction
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
                Ok(VerificationRunRow {
                    verification_id: row.get(0)?,
                    candidate_token: row.get(1)?,
                    owner_plan_id: row.get(2)?,
                    owner_plan_digest: row.get(3)?,
                    verification_generation: row.get(4)?,
                    candidate_generation: row.get(5)?,
                    application_inventory_revision: row.get(6)?,
                    authority_state_revision: row.get(7)?,
                    authority_epoch: row.get(8)?,
                    process_owner_epoch: row.get(9)?,
                    artifact_count: row.get(10)?,
                    artifact_bytes: row.get(11)?,
                    expected_artifact_set_digest: row.get(12)?,
                    file_set_binding_digest: row.get(13)?,
                    state: row.get(14)?,
                    prepared_at_ms: row.get(15)?,
                    resolved_at_ms: row.get(16)?,
                    resolution_reason: row.get(17)?,
                    result_json: row.get(18)?,
                    result_digest: row.get(19)?,
                    mismatch_ordinal: row.get(20)?,
                    observed_digest: row.get(21)?,
                })
            },
        )
        .optional()
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RUN_READ")?
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_IDENTITY_CHANGED"))
}

fn validate_run_identity(
    run: &VerificationRunRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<()> {
    if run.verification_id != key.verification_id()
        || run.candidate_token != key.candidate_token()
        || run.owner_plan_id != key.owner_plan_id()
        || run.owner_plan_digest != key.owner_plan_digest()
        || run.verification_generation != key.verification_generation()
        || run.candidate_generation != key.candidate_generation()
        || run.application_inventory_revision != key.application_inventory_revision()
        || run.authority_state_revision != key.authority_state_revision()
        || run.authority_epoch != key.authority_epoch()
        || run.process_owner_epoch != key.process_owner_epoch()
        || usize::try_from(run.artifact_count).ok() != Some(key.artifact_count())
        || run.artifact_bytes != key.artifact_bytes()
        || run.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || run.file_set_binding_digest != key.file_set_binding_digest()
        || run.prepared_at_ms != key.prepared_at_ms()
        || jcs_sha256_hex(&run.candidate_token)? != key.candidate_token_digest()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_IDENTITY_CHANGED");
    }
    Ok(())
}

fn classify_run(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    run: &VerificationRunRow,
    closure: Option<&CandidateClosureSnapshot>,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<ComputePluginCandidateVerificationOutcome> {
    if authority.trusted_time_high_water_ms < key.prepared_at_ms() {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_TIME_CHANGED");
    }
    match run.state.as_str() {
        "prepared" => classify_prepared(
            transaction,
            authority,
            run,
            closure.ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_CLOSURE_MISSING")
            })?,
            key,
        ),
        "aborted" => classify_terminal(
            transaction,
            authority,
            run,
            key,
            CandidateVerificationTerminalKind::Aborted,
            ComputePluginCandidateVerificationOutcomeKind::Aborted,
        ),
        "revoked" => classify_terminal(
            transaction,
            authority,
            run,
            key,
            CandidateVerificationTerminalKind::Revoked,
            ComputePluginCandidateVerificationOutcomeKind::Revoked,
        ),
        "verified" | "rejected" => {
            bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESULT_UNSUPPORTED")
        }
        _ => bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_STATE_CORRUPT"),
    }
}

fn classify_prepared(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    run: &VerificationRunRow,
    closure: &CandidateClosureSnapshot,
    key: &ComputePluginCandidateVerificationRecoveryKey,
) -> Result<ComputePluginCandidateVerificationOutcome> {
    if authority.state_revision != key.authority_state_revision()
        || authority.inventory_revision != key.execution_inventory_revision()
        || authority.inventory_digest != key.inventory_digest()
        || authority.authority_epoch != key.authority_epoch()
        || authority.process_owner_epoch != key.process_owner_epoch()
        || closure.durable_closure_digest != key.durable_candidate_closure_digest()
        || closure.expected_artifact_set_digest != key.expected_artifact_set_digest()
        || closure.artifact_count != key.artifact_count()
        || closure.artifact_bytes != key.artifact_bytes()
        || run.resolved_at_ms.is_some()
        || run.resolution_reason.is_some()
        || run.result_json.is_some()
        || run.result_digest.is_some()
        || run.mismatch_ordinal.is_some()
        || run.observed_digest.is_some()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_PREPARED_CORRUPT");
    }
    let prepared_fetches = transaction
        .query_row(
            "SELECT COUNT(*) FROM fetch_claims WHERE candidate_token = ?1 AND state = 'prepared'",
            [key.candidate_token()],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_PREPARED_FETCH_READ")?;
    if prepared_fetches != 0 {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_PREPARED_FETCH_CHANGED");
    }
    Ok(ComputePluginCandidateVerificationOutcome::from_store(
        ComputePluginCandidateVerificationOutcomeKind::Prepared,
        key,
        None,
        None,
        None,
    ))
}

fn classify_terminal(
    transaction: &Transaction<'_>,
    authority: &VerificationRecoveryAuthorityRow,
    run: &VerificationRunRow,
    key: &ComputePluginCandidateVerificationRecoveryKey,
    terminal_kind: CandidateVerificationTerminalKind,
    outcome_kind: ComputePluginCandidateVerificationOutcomeKind,
) -> Result<ComputePluginCandidateVerificationOutcome> {
    let resolved_at_ms = run
        .resolved_at_ms
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESOLUTION_MISSING"))?;
    let reason = run
        .resolution_reason
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_REASON_MISSING"))?;
    let result_json = run
        .result_json
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_RESULT_MISSING"))?;
    let result_digest = run
        .result_digest
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_DIGEST_MISSING"))?;
    let parsed_reason = parse_terminal_reason(terminal_kind, reason)?;
    if resolved_at_ms < key.prepared_at_ms()
        || resolved_at_ms > authority.trusted_time_high_water_ms
        || run.mismatch_ordinal.is_some()
        || run.observed_digest.is_some()
    {
        bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_TERMINAL_CORRUPT");
    }
    validate_candidate_verification_terminal_result(
        terminal_kind,
        parsed_reason,
        resolved_at_ms,
        result_json,
        result_digest,
    )?;
    if terminal_kind == CandidateVerificationTerminalKind::Revoked {
        revocation::validate_durable_state(transaction, authority, run, key, parsed_reason)?;
    }
    Ok(ComputePluginCandidateVerificationOutcome::from_store(
        outcome_kind,
        key,
        Some(resolved_at_ms),
        Some(parsed_reason),
        Some(result_digest.to_string()),
    ))
}

fn parse_terminal_reason(
    kind: CandidateVerificationTerminalKind,
    reason: &str,
) -> Result<&'static str> {
    match (kind, reason) {
        (CandidateVerificationTerminalKind::Aborted, "verification_aborted") => {
            Ok("verification_aborted")
        }
        (CandidateVerificationTerminalKind::Aborted, "authority_recovery") => {
            Ok("authority_recovery")
        }
        (CandidateVerificationTerminalKind::Revoked, "authority_epoch_advanced_by_keyring") => {
            Ok("authority_epoch_advanced_by_keyring")
        }
        (CandidateVerificationTerminalKind::Revoked, "authority_epoch_advanced_by_plan") => {
            Ok("authority_epoch_advanced_by_plan")
        }
        (CandidateVerificationTerminalKind::Revoked, "process_owner_epoch_advanced") => {
            Ok("process_owner_epoch_advanced")
        }
        (CandidateVerificationTerminalKind::Revoked, "candidate_released_by_plan") => {
            Ok("candidate_released_by_plan")
        }
        _ => bail!("COMPUTE_PLUGIN_VERIFICATION_OUTCOME_REASON_CORRUPT"),
    }
}
