use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::federation_historical_causal_reference::{
        build_settlement_release_source_carrier, FederationHistoricalLineageKindV1,
        SettlementReleaseGateV1, SettlementReleaseSourceLineageV1,
    },
    store::{
        compute_attempt_settlement_challenge_resolutions::compute_attempt_historical_settlement_challenge_resolution_by_challenge_on,
        compute_attempt_settlement_challenges::compute_attempt_historical_settlement_challenge_by_lease_on,
        compute_attempt_settlement_corrections::compute_attempt_historical_settlement_correction_by_resolution_on,
        compute_attempt_settlement_releases::ComputeSettlementReleaseReceipt,
        compute_attempt_settlements::compute_attempt_historical_settlement_by_lease_on,
    },
};

use super::{
    release_refs::{
        settlement_challenge_ref, settlement_challenge_resolution_action,
        settlement_challenge_resolution_ref, settlement_correction_posting_ref,
        settlement_correction_ref, settlement_release_posting_ref, settlement_release_ref,
        source_settlement_posting_ref, validate_settlement_release_source_links,
        SettlementReleaseSourceLinkFacts,
    },
    settlement::resolve_settlement_source_lineage_on,
    source_refs::settlement_ref,
    ValidatedFederationHistoricalLineage,
};

pub(super) fn resolve_settlement_release_source_lineage_on(
    conn: &Connection,
    release: &ComputeSettlementReleaseReceipt,
) -> Result<ValidatedFederationHistoricalLineage> {
    let settlement = compute_attempt_historical_settlement_by_lease_on(conn, &release.lease_id)?
        .ok_or_else(|| anyhow!("Settlement Release source v195 root does not exist"))?;
    if settlement.lease_id != release.lease_id
        || settlement.settlement.settlement_receipt_id != release.settlement_receipt_id
        || settlement.event_digest != release.settlement_event_digest
        || settlement.posting_id != release.source_posting_id
        || settlement.posting_digest != release.source_posting_digest
        || settlement.settlement.consumer_account_id != release.consumer_account_id
        || settlement.settlement.provider_account_id != release.provider_account_id
    {
        bail!("Settlement Release and v195 retained owner bodies do not form one exact chain");
    }

    let rebuilt_settlement = resolve_settlement_source_lineage_on(
        conn,
        &settlement.settlement.settlement_receipt_id,
        &settlement.settlement.settlement_receipt_digest,
        &settlement.event_digest,
    )?;
    if rebuilt_settlement.kind() != FederationHistoricalLineageKindV1::SettlementSourceV1 {
        bail!("Settlement Release rebuilt a non-settlement historical carrier");
    }
    let (settlement_lineage_digest, access_scope) =
        rebuilt_settlement.into_lineage_digest_and_access_scope();

    let attempt_settlement = settlement_ref(
        &settlement.settlement.settlement_receipt_id,
        &settlement.settlement.settlement_receipt_digest,
        &settlement.event_digest,
    );
    let source_settlement_posting =
        source_settlement_posting_ref(&release.source_posting_id, &release.source_posting_digest);
    let release_gate = resolve_release_gate_on(conn, release)?;
    let settlement_release = settlement_release_ref(&release.release_id, &release.event_digest);
    let release_posting = settlement_release_posting_ref(
        &release.release_posting_id,
        &release.release_posting_digest,
    );
    let lineage = SettlementReleaseSourceLineageV1 {
        attempt_settlement: attempt_settlement.clone(),
        settlement_lineage_digest: settlement_lineage_digest.clone(),
        source_settlement_posting: source_settlement_posting.clone(),
        release_gate: release_gate.clone(),
        settlement_release: settlement_release.clone(),
        release_posting: release_posting.clone(),
    };
    let facts = SettlementReleaseSourceLinkFacts {
        lineage,
        audited_attempt_settlement: attempt_settlement,
        rebuilt_settlement_lineage_digest: settlement_lineage_digest,
        audited_source_settlement_posting: source_settlement_posting,
        audited_release_gate: release_gate,
        audited_settlement_release: settlement_release,
        audited_release_posting: release_posting,
        settlement_lease_id: settlement.lease_id,
        release_lease_id: release.lease_id.clone(),
        settlement_consumer_account_id: settlement.settlement.consumer_account_id,
        release_consumer_account_id: release.consumer_account_id.clone(),
        settlement_provider_account_id: settlement.settlement.provider_account_id,
        release_provider_account_id: release.provider_account_id.clone(),
    };
    validate_settlement_release_source_links(&facts)?;
    ValidatedFederationHistoricalLineage::from_carrier(
        build_settlement_release_source_carrier(facts.lineage)?,
        access_scope,
    )
}

fn resolve_release_gate_on(
    conn: &Connection,
    release: &ComputeSettlementReleaseReceipt,
) -> Result<SettlementReleaseGateV1> {
    match release.challenge_gate.status.as_str() {
        "none" => resolve_no_challenge_gate_on(conn, release),
        "rejected" | "withdrawn" => resolve_terminal_challenge_gate_on(conn, release),
        "accepted_corrected" => resolve_corrected_challenge_gate_on(conn, release),
        "open" | "accepted" => bail!("Settlement Release retained a blocked challenge gate"),
        _ => bail!("Settlement Release retained an unknown challenge gate"),
    }
}

fn resolve_no_challenge_gate_on(
    conn: &Connection,
    release: &ComputeSettlementReleaseReceipt,
) -> Result<SettlementReleaseGateV1> {
    if release.challenge_gate.blocked
        || release.challenge_gate.correction_required
        || release.challenge_gate.challenge_id.is_some()
        || release.challenge_gate.challenge_event_digest.is_some()
        || release.challenge_gate.resolution_id.is_some()
        || release.challenge_gate.resolution_event_digest.is_some()
        || release.challenge_gate.correction_id.is_some()
        || release.challenge_gate.correction_event_digest.is_some()
        || compute_attempt_historical_settlement_challenge_by_lease_on(conn, &release.lease_id)?
            .is_some()
    {
        bail!("Settlement Release no-challenge gate is not exact");
    }
    Ok(SettlementReleaseGateV1::NoChallenge {
        challenge_gate_digest: release.challenge_gate_digest.clone(),
    })
}

fn resolve_terminal_challenge_gate_on(
    conn: &Connection,
    release: &ComputeSettlementReleaseReceipt,
) -> Result<SettlementReleaseGateV1> {
    let challenge = historical_challenge_on(conn, release)?;
    let resolution = historical_resolution_on(conn, release, &challenge.challenge_id)?;
    if release.challenge_gate.blocked
        || release.challenge_gate.correction_required
        || release.challenge_gate.status != resolution.action
        || !matches!(resolution.action.as_str(), "rejected" | "withdrawn")
        || release.challenge_gate.correction_id.is_some()
        || release.challenge_gate.correction_event_digest.is_some()
        || compute_attempt_historical_settlement_correction_by_resolution_on(
            conn,
            &resolution.resolution_id,
        )?
        .is_some()
    {
        bail!("Settlement Release resolved challenge gate is not exact");
    }
    Ok(SettlementReleaseGateV1::ResolvedChallenge {
        challenge_gate_digest: release.challenge_gate_digest.clone(),
        resolution_action: settlement_challenge_resolution_action(&resolution.action)?,
        challenge: settlement_challenge_ref(&challenge.challenge_id, &challenge.event_digest),
        resolution: settlement_challenge_resolution_ref(
            &resolution.resolution_id,
            &resolution.event_digest,
        ),
    })
}

fn resolve_corrected_challenge_gate_on(
    conn: &Connection,
    release: &ComputeSettlementReleaseReceipt,
) -> Result<SettlementReleaseGateV1> {
    let challenge = historical_challenge_on(conn, release)?;
    let resolution = historical_resolution_on(conn, release, &challenge.challenge_id)?;
    let correction = compute_attempt_historical_settlement_correction_by_resolution_on(
        conn,
        &resolution.resolution_id,
    )?
    .ok_or_else(|| anyhow!("Settlement Release accepted gate lacks v199 correction"))?;
    if release.challenge_gate.blocked
        || release.challenge_gate.correction_required
        || resolution.action != "accepted"
        || release.challenge_gate.correction_id.as_deref()
            != Some(correction.correction_id.as_str())
        || release.challenge_gate.correction_event_digest.as_deref()
            != Some(correction.event_digest.as_str())
        || correction.lease_id != release.lease_id
        || correction.settlement_receipt_id != release.settlement_receipt_id
        || correction.settlement_event_digest != release.settlement_event_digest
        || correction.challenge_id != challenge.challenge_id
        || correction.challenge_event_digest != challenge.event_digest
        || correction.resolution_id != resolution.resolution_id
        || correction.resolution_event_digest != resolution.event_digest
    {
        bail!("Settlement Release accepted-corrected gate is not exact");
    }
    Ok(SettlementReleaseGateV1::AcceptedCorrected {
        challenge_gate_digest: release.challenge_gate_digest.clone(),
        challenge: settlement_challenge_ref(&challenge.challenge_id, &challenge.event_digest),
        resolution: settlement_challenge_resolution_ref(
            &resolution.resolution_id,
            &resolution.event_digest,
        ),
        correction: settlement_correction_ref(&correction.correction_id, &correction.event_digest),
        correction_posting: settlement_correction_posting_ref(
            &correction.posting_id,
            &correction.posting_digest,
        ),
    })
}

fn historical_challenge_on(
    conn: &Connection,
    release: &ComputeSettlementReleaseReceipt,
) -> Result<super::super::compute_attempt_settlement_challenges::ComputeSettlementChallengeReceipt>
{
    let challenge =
        compute_attempt_historical_settlement_challenge_by_lease_on(conn, &release.lease_id)?
            .ok_or_else(|| anyhow!("Settlement Release challenge gate lacks v196 challenge"))?;
    if release.challenge_gate.challenge_id.as_deref() != Some(challenge.challenge_id.as_str())
        || release.challenge_gate.challenge_event_digest.as_deref()
            != Some(challenge.event_digest.as_str())
        || challenge.settlement_receipt_id != release.settlement_receipt_id
        || challenge.settlement_event_digest != release.settlement_event_digest
        || challenge.lease_id != release.lease_id
    {
        bail!("Settlement Release challenge gate and v196 differ");
    }
    Ok(challenge)
}

fn historical_resolution_on(
    conn: &Connection,
    release: &ComputeSettlementReleaseReceipt,
    challenge_id: &str,
) -> Result<
    super::super::compute_attempt_settlement_challenge_resolutions::ComputeSettlementChallengeResolutionReceipt,
>{
    let resolution = compute_attempt_historical_settlement_challenge_resolution_by_challenge_on(
        conn,
        challenge_id,
    )?
    .ok_or_else(|| anyhow!("Settlement Release challenge gate lacks v197 resolution"))?;
    if release.challenge_gate.resolution_id.as_deref() != Some(resolution.resolution_id.as_str())
        || release.challenge_gate.resolution_event_digest.as_deref()
            != Some(resolution.event_digest.as_str())
        || resolution.challenge_id != challenge_id
        || resolution.settlement_receipt_id != release.settlement_receipt_id
        || resolution.settlement_event_digest != release.settlement_event_digest
        || resolution.lease_id != release.lease_id
    {
        bail!("Settlement Release challenge gate and v197 differ");
    }
    Ok(resolution)
}
