use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::Transaction;

use super::{
    read_resolution_download, require_prepared_claim, require_terminal_claim, terminalize_claim,
    validate_abort_authority, validate_claim_identity, validate_resolution_download, ClaimIdentity,
};
use crate::node_agent_compute_plugin_host::fetch_contract::ValidatedComputePluginCursorDamagePermit;

use super::super::super::{keyring_snapshot::advance_trusted_time, ComputePluginFetchProcessFence};

pub(super) fn fail_validated_cursor_damage(
    transaction: &Transaction<'_>,
    process_fence: &ComputePluginFetchProcessFence,
    trusted_now: DateTime<Utc>,
    permit: ValidatedComputePluginCursorDamagePermit<'_>,
) -> Result<()> {
    if !permit.reason().is_cursor_damage() {
        bail!("COMPUTE_PLUGIN_FETCH_CURSOR_DAMAGE_REASON_INVALID");
    }
    let claim = ClaimIdentity::from_abort(permit.abort_permit());
    validate_claim_identity(&claim)?;
    let download = read_resolution_download(transaction, &claim)?;
    validate_resolution_download(&claim, &download, None)?;
    require_prepared_claim(transaction, &claim, &download.candidate_token)?;
    let trusted_now_ms = trusted_now.timestamp_millis();
    let state = validate_abort_authority(transaction, process_fence, &claim, trusted_now_ms)?;
    advance_trusted_time(transaction, &state, trusted_now_ms)?;
    terminalize_claim(
        transaction,
        &claim,
        &download.candidate_token,
        "aborted",
        permit.reason().as_str(),
        trusted_now_ms,
    )?;
    require_terminal_claim(
        transaction,
        &claim,
        &download.candidate_token,
        "aborted",
        permit.reason().as_str(),
        trusted_now_ms,
    )?;

    let mut expected = download.clone();
    expected.state = "failed".to_string();
    expected.updated_at_ms = trusted_now_ms;
    if read_resolution_download(transaction, &claim)? != expected {
        bail!("COMPUTE_PLUGIN_FETCH_CURSOR_DAMAGE_POST_WRITE_MISMATCH");
    }
    Ok(())
}
