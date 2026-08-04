use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Transaction};

use super::{
    fetch_claim_revocation::revoke_for_process_owner_epoch_advance, ComputePluginLocalAuthority,
};
use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ComputePluginFetchProcessFence {
    installation_id_digest: String,
    process_owner_epoch: i64,
    acquired_at_ms: i64,
}

impl ComputePluginFetchProcessFence {
    pub(super) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(super) fn process_owner_epoch(&self) -> i64 {
        self.process_owner_epoch
    }

    pub(super) fn acquired_at_ms(&self) -> i64 {
        self.acquired_at_ms
    }
}

struct ProcessOwnershipState {
    installation_id_digest: String,
    state_revision: i64,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_time_high_water_ms: i64,
    updated_at_ms: i64,
    clock_status: String,
}

impl ComputePluginLocalAuthority {
    /// Call once after the NodeAgent instance lock and trusted clock are established. A commit
    /// error has an uncertain outcome: callers must restart recovery rather than retry in-process.
    pub(crate) fn acquire_fetch_process_fence(
        &self,
        trusted_now: DateTime<Utc>,
    ) -> Result<ComputePluginFetchProcessFence> {
        let trusted_now_ms = trusted_now.timestamp_millis();
        self.with_immediate(|transaction| {
            let state = read_process_ownership_state(transaction)?;
            validate_process_ownership_state(&state, trusted_now_ms)?;
            let next_state_revision = state
                .state_revision
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_STATE_EXHAUSTED"))?;
            let next_process_owner_epoch = state
                .process_owner_epoch
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_PROCESS_OWNER_EPOCH_EXHAUSTED"))?;
            revoke_for_process_owner_epoch_advance(
                transaction,
                state.authority_epoch,
                state.process_owner_epoch,
                next_process_owner_epoch,
                trusted_now_ms,
            )?;
            update_process_ownership(
                transaction,
                &state,
                next_state_revision,
                next_process_owner_epoch,
                trusted_now_ms,
            )?;
            verify_process_ownership(
                transaction,
                &state.installation_id_digest,
                next_state_revision,
                state.authority_epoch,
                next_process_owner_epoch,
                trusted_now_ms,
            )?;
            Ok(ComputePluginFetchProcessFence {
                installation_id_digest: state.installation_id_digest,
                process_owner_epoch: next_process_owner_epoch,
                acquired_at_ms: trusted_now_ms,
            })
        })
    }
}

fn read_process_ownership_state(transaction: &Transaction<'_>) -> Result<ProcessOwnershipState> {
    transaction
        .query_row(
            r#"SELECT installation_id_digest, state_revision, authority_epoch,
                process_owner_epoch, trusted_time_high_water_ms, updated_at_ms, clock_status
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok(ProcessOwnershipState {
                    installation_id_digest: row.get(0)?,
                    state_revision: row.get(1)?,
                    authority_epoch: row.get(2)?,
                    process_owner_epoch: row.get(3)?,
                    trusted_time_high_water_ms: row.get(4)?,
                    updated_at_ms: row.get(5)?,
                    clock_status: row.get(6)?,
                })
            },
        )
        .context("COMPUTE_PLUGIN_PROCESS_OWNER_STATE_READ")
}

fn validate_process_ownership_state(
    state: &ProcessOwnershipState,
    trusted_now_ms: i64,
) -> Result<()> {
    if !is_sha256(&state.installation_id_digest)
        || state.state_revision < 0
        || state.authority_epoch < 0
        || state.process_owner_epoch < 0
        || state.trusted_time_high_water_ms < 0
        || state.updated_at_ms < 0
        || state.clock_status != "trusted"
        || trusted_now_ms < state.trusted_time_high_water_ms
        || trusted_now_ms < state.updated_at_ms
    {
        bail!("COMPUTE_PLUGIN_PROCESS_OWNER_STATE_INVALID");
    }
    Ok(())
}

fn update_process_ownership(
    transaction: &Transaction<'_>,
    state: &ProcessOwnershipState,
    next_state_revision: i64,
    next_process_owner_epoch: i64,
    trusted_now_ms: i64,
) -> Result<()> {
    let updated = transaction
        .execute(
            r#"UPDATE authority_meta SET
                state_revision = :next_state_revision,
                process_owner_epoch = :next_process_owner_epoch,
                trusted_time_high_water_ms = :trusted_now,
                clock_status = 'trusted',
                updated_at_ms = :trusted_now
            WHERE singleton = 1
              AND installation_id_digest = :installation_id_digest
              AND state_revision = :old_state_revision
              AND authority_epoch = :authority_epoch
              AND process_owner_epoch = :old_process_owner_epoch
              AND trusted_time_high_water_ms = :old_trusted_time
              AND updated_at_ms = :old_updated_at
              AND clock_status = 'trusted'"#,
            named_params! {
                ":next_state_revision": next_state_revision,
                ":next_process_owner_epoch": next_process_owner_epoch,
                ":trusted_now": trusted_now_ms,
                ":installation_id_digest": &state.installation_id_digest,
                ":old_state_revision": state.state_revision,
                ":authority_epoch": state.authority_epoch,
                ":old_process_owner_epoch": state.process_owner_epoch,
                ":old_trusted_time": state.trusted_time_high_water_ms,
                ":old_updated_at": state.updated_at_ms,
            },
        )
        .context("COMPUTE_PLUGIN_PROCESS_OWNER_UPDATE")?;
    if updated != 1 {
        bail!("COMPUTE_PLUGIN_PROCESS_OWNER_CAS");
    }
    Ok(())
}

fn verify_process_ownership(
    transaction: &Transaction<'_>,
    installation_id_digest: &str,
    state_revision: i64,
    authority_epoch: i64,
    process_owner_epoch: i64,
    trusted_now_ms: i64,
) -> Result<()> {
    let matches = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM authority_meta AS meta
            WHERE meta.singleton = 1
              AND meta.installation_id_digest = ?1
              AND meta.state_revision = ?2
              AND meta.authority_epoch = ?3
              AND meta.process_owner_epoch = ?4
              AND meta.trusted_time_high_water_ms = ?5
              AND meta.updated_at_ms = ?5
              AND meta.clock_status = 'trusted'
              AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')"#,
            params![
                installation_id_digest,
                state_revision,
                authority_epoch,
                process_owner_epoch,
                trusted_now_ms,
            ],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_PROCESS_OWNER_VERIFY")?;
    if matches != 1 {
        bail!("COMPUTE_PLUGIN_PROCESS_OWNER_VERIFY_MISMATCH");
    }
    Ok(())
}
