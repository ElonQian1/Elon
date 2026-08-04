use anyhow::{bail, Result};
use rusqlite::{params, Connection};

use super::{
    super::{
        compute_attempt_settlement_challenge_resolutions::compute_settlement_challenge_resolution_on,
        compute_attempt_settlement_challenges::compute_settlement_challenge_on,
        compute_attempt_settlements::compute_attempt_settlement_on,
    },
    compute_settlement_correction_by_resolution_on, ComputePendingSettlementCorrectionCandidate,
};

pub(super) fn build_pending_correction_candidate_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputePendingSettlementCorrectionCandidate> {
    let settlement = compute_attempt_settlement_on(conn, lease_id)?;
    let challenge = compute_settlement_challenge_on(conn, lease_id)?;
    let resolution = compute_settlement_challenge_resolution_on(conn, lease_id)?;
    if resolution.action != "accepted"
        || !resolution.correction_required
        || resolution.challenge_id != challenge.challenge_id
        || resolution.challenge_event_digest != challenge.event_digest
        || resolution.settlement_receipt_id != settlement.settlement.settlement_receipt_id
        || resolution.settlement_event_digest != settlement.event_digest
        || challenge.settlement_receipt_id != settlement.settlement.settlement_receipt_id
        || challenge.settlement_event_digest != settlement.event_digest
        || settlement.settlement.balance_state != "pending"
    {
        bail!("待纠正候选的 accepted 决议、挑战或 Settlement Receipt 不匹配");
    }
    if compute_settlement_correction_by_resolution_on(conn, &resolution.resolution_id)?.is_some() {
        bail!("待纠正候选已经存在 v199 Correction Receipt");
    }
    let already_released = conn.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM compute_settlement_releases
            WHERE settlement_receipt_id=?1
         )",
        params![settlement.settlement.settlement_receipt_id],
        |row| row.get::<_, bool>(0),
    )?;
    if already_released {
        bail!("待纠正候选已经释放到 available");
    }

    Ok(ComputePendingSettlementCorrectionCandidate {
        settlement,
        challenge,
        resolution,
        balance_effect: "read_only_no_balance_change",
        settlement_release_effect: "blocked_until_v199_correction",
        external_payment_effect: "none",
    })
}
