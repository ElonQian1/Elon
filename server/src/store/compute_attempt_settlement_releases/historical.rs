use anyhow::{bail, Result};
use rusqlite::Connection;

use super::super::{
    compute_attempt_settlement_challenge_resolutions::compute_attempt_historical_settlement_challenge_resolution_by_challenge_on,
    compute_attempt_settlement_challenges::{
        compute_attempt_historical_settlement_challenge_by_lease_on, ComputeSettlementChallengeGate,
    },
    compute_attempt_settlement_corrections::{
        compute_attempt_historical_settlement_correction_by_resolution_on,
        ComputeSettlementCorrectionReceipt,
    },
    compute_attempt_settlements::ComputeAttemptSettlementReceipt,
};

pub(super) struct HistoricalReleaseGateFacts {
    pub(super) gate: ComputeSettlementChallengeGate,
    pub(super) correction: Option<ComputeSettlementCorrectionReceipt>,
}

pub(super) fn rebuild_historical_release_gate_on(
    conn: &Connection,
    lease_id: &str,
    settlement_receipt_id: &str,
    settlement_event_digest: &str,
) -> Result<HistoricalReleaseGateFacts> {
    let Some(challenge) =
        compute_attempt_historical_settlement_challenge_by_lease_on(conn, lease_id)?
    else {
        return Ok(HistoricalReleaseGateFacts {
            gate: ComputeSettlementChallengeGate {
                status: "none".to_string(),
                blocked: false,
                correction_required: false,
                challenge_id: None,
                challenge_event_digest: None,
                resolution_id: None,
                resolution_event_digest: None,
                correction_id: None,
                correction_event_digest: None,
            },
            correction: None,
        });
    };
    if challenge.lease_id != lease_id
        || challenge.settlement_receipt_id != settlement_receipt_id
        || challenge.settlement_event_digest != settlement_event_digest
    {
        bail!("v198 历史门卫引用的 v196 挑战不属于同一 Settlement");
    }

    let Some(resolution) =
        compute_attempt_historical_settlement_challenge_resolution_by_challenge_on(
            conn,
            &challenge.challenge_id,
        )?
    else {
        return Ok(HistoricalReleaseGateFacts {
            gate: ComputeSettlementChallengeGate {
                status: "open".to_string(),
                blocked: true,
                correction_required: false,
                challenge_id: Some(challenge.challenge_id),
                challenge_event_digest: Some(challenge.event_digest),
                resolution_id: None,
                resolution_event_digest: None,
                correction_id: None,
                correction_event_digest: None,
            },
            correction: None,
        });
    };
    if resolution.lease_id != lease_id
        || resolution.challenge_id != challenge.challenge_id
        || resolution.challenge_event_digest != challenge.event_digest
        || resolution.settlement_receipt_id != settlement_receipt_id
        || resolution.settlement_event_digest != settlement_event_digest
    {
        bail!("v198 历史门卫引用的 v197 决议不属于同一 Settlement");
    }

    if resolution.action == "accepted" {
        let correction = compute_attempt_historical_settlement_correction_by_resolution_on(
            conn,
            &resolution.resolution_id,
        )?;
        if let Some(correction) = correction {
            if correction.lease_id != lease_id
                || correction.challenge_id != challenge.challenge_id
                || correction.challenge_event_digest != challenge.event_digest
                || correction.resolution_id != resolution.resolution_id
                || correction.resolution_event_digest != resolution.event_digest
                || correction.settlement_receipt_id != settlement_receipt_id
                || correction.settlement_event_digest != settlement_event_digest
            {
                bail!("v198 历史门卫引用的 v199 纠正不属于同一 Settlement");
            }
            return Ok(HistoricalReleaseGateFacts {
                gate: ComputeSettlementChallengeGate {
                    status: "accepted_corrected".to_string(),
                    blocked: false,
                    correction_required: false,
                    challenge_id: Some(challenge.challenge_id),
                    challenge_event_digest: Some(challenge.event_digest),
                    resolution_id: Some(resolution.resolution_id),
                    resolution_event_digest: Some(resolution.event_digest),
                    correction_id: Some(correction.correction_id.clone()),
                    correction_event_digest: Some(correction.event_digest.clone()),
                },
                correction: Some(correction),
            });
        }
    }

    let (blocked, correction_required) = match resolution.action.as_str() {
        "accepted" => (true, true),
        "rejected" | "withdrawn" => (false, false),
        _ => bail!("v198 历史门卫包含未知 v197 终态"),
    };
    Ok(HistoricalReleaseGateFacts {
        gate: ComputeSettlementChallengeGate {
            status: resolution.action,
            blocked,
            correction_required,
            challenge_id: Some(challenge.challenge_id),
            challenge_event_digest: Some(challenge.event_digest),
            resolution_id: Some(resolution.resolution_id),
            resolution_event_digest: Some(resolution.event_digest),
            correction_id: None,
            correction_event_digest: None,
        },
        correction: None,
    })
}

pub(super) fn historical_release_amounts(
    settlement: &ComputeAttemptSettlementReceipt,
    facts: &HistoricalReleaseGateFacts,
) -> Result<(i64, i64)> {
    match facts.gate.status.as_str() {
        "none" | "rejected" | "withdrawn" => {
            if facts.gate.blocked
                || facts.gate.correction_required
                || facts.correction.is_some()
                || facts.gate.correction_id.is_some()
                || facts.gate.correction_event_digest.is_some()
            {
                bail!("v198 历史非纠正门卫携带了阻断或纠正事实");
            }
            Ok((
                settlement.settlement.amounts.provider_payable_micros,
                settlement.settlement.amounts.platform_margin_micros,
            ))
        }
        "accepted_corrected" => {
            let correction = facts
                .correction
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("v198 accepted_corrected 历史门卫缺少 v199"))?;
            if facts.gate.blocked
                || facts.gate.correction_required
                || facts.gate.correction_id.as_deref() != Some(correction.correction_id.as_str())
                || facts.gate.correction_event_digest.as_deref()
                    != Some(correction.event_digest.as_str())
            {
                bail!("v198 accepted_corrected 历史门卫与 v199 不一致");
            }
            Ok((
                correction.corrected_provider_payable_micros,
                correction.corrected_platform_margin_micros,
            ))
        }
        "open" | "accepted" => bail!("v198 历史释放不能绑定仍阻断的挑战门卫"),
        _ => bail!("v198 历史释放包含未知挑战门卫"),
    }
}
