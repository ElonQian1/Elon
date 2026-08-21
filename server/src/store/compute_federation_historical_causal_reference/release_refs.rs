use anyhow::{bail, Result};

use crate::compute_federation::federation_historical_causal_reference::{
    AttemptSettlementRef, SettlementChallengeRef, SettlementChallengeResolutionActionV1,
    SettlementChallengeResolutionRef, SettlementCorrectionPostingRef, SettlementCorrectionRef,
    SettlementReleaseGateV1, SettlementReleasePostingRef, SettlementReleaseRef,
    SettlementReleaseSourceLineageV1, SettlementSourcePostingRef,
};

#[derive(Clone)]
pub(super) struct SettlementReleaseSourceLinkFacts {
    pub(super) lineage: SettlementReleaseSourceLineageV1,
    pub(super) audited_attempt_settlement: AttemptSettlementRef,
    pub(super) rebuilt_settlement_lineage_digest: String,
    pub(super) audited_source_settlement_posting: SettlementSourcePostingRef,
    pub(super) audited_release_gate: SettlementReleaseGateV1,
    pub(super) audited_settlement_release: SettlementReleaseRef,
    pub(super) audited_release_posting: SettlementReleasePostingRef,
    pub(super) settlement_lease_id: String,
    pub(super) release_lease_id: String,
    pub(super) settlement_consumer_account_id: String,
    pub(super) release_consumer_account_id: String,
    pub(super) settlement_provider_account_id: String,
    pub(super) release_provider_account_id: String,
}

pub(super) fn validate_settlement_release_source_links(
    facts: &SettlementReleaseSourceLinkFacts,
) -> Result<()> {
    let lineage = &facts.lineage;
    if lineage.attempt_settlement != facts.audited_attempt_settlement
        || lineage.settlement_lineage_digest != facts.rebuilt_settlement_lineage_digest
        || lineage.source_settlement_posting != facts.audited_source_settlement_posting
        || lineage.release_gate != facts.audited_release_gate
        || lineage.settlement_release != facts.audited_settlement_release
        || lineage.release_posting != facts.audited_release_posting
        || facts.settlement_lease_id != facts.release_lease_id
        || facts.settlement_consumer_account_id != facts.release_consumer_account_id
        || facts.settlement_provider_account_id != facts.release_provider_account_id
    {
        bail!("settlement release source 的 v195-v199/v198 历史链不一致");
    }
    Ok(())
}

pub(super) fn source_settlement_posting_ref(
    posting_id: &str,
    posting_digest: &str,
) -> SettlementSourcePostingRef {
    SettlementSourcePostingRef {
        settlement_posting_id: posting_id.to_string(),
        settlement_posting_digest: posting_digest.to_string(),
    }
}

pub(super) fn settlement_release_ref(release_id: &str, event_digest: &str) -> SettlementReleaseRef {
    SettlementReleaseRef {
        settlement_release_id: release_id.to_string(),
        settlement_release_event_digest: event_digest.to_string(),
    }
}

pub(super) fn settlement_release_posting_ref(
    posting_id: &str,
    posting_digest: &str,
) -> SettlementReleasePostingRef {
    SettlementReleasePostingRef {
        settlement_release_posting_id: posting_id.to_string(),
        settlement_release_posting_digest: posting_digest.to_string(),
    }
}

pub(super) fn settlement_challenge_ref(
    challenge_id: &str,
    event_digest: &str,
) -> SettlementChallengeRef {
    SettlementChallengeRef {
        settlement_challenge_id: challenge_id.to_string(),
        settlement_challenge_event_digest: event_digest.to_string(),
    }
}

pub(super) fn settlement_challenge_resolution_ref(
    resolution_id: &str,
    event_digest: &str,
) -> SettlementChallengeResolutionRef {
    SettlementChallengeResolutionRef {
        settlement_challenge_resolution_id: resolution_id.to_string(),
        settlement_challenge_resolution_event_digest: event_digest.to_string(),
    }
}

pub(super) fn settlement_challenge_resolution_action(
    action: &str,
) -> Result<SettlementChallengeResolutionActionV1> {
    match action {
        "rejected" => SettlementChallengeResolutionActionV1::Rejected,
        "withdrawn" => SettlementChallengeResolutionActionV1::Withdrawn,
        _ => bail!("settlement release source 引用了未知 v197 action"),
    }
}

pub(super) fn settlement_correction_ref(
    correction_id: &str,
    event_digest: &str,
) -> SettlementCorrectionRef {
    SettlementCorrectionRef {
        settlement_correction_id: correction_id.to_string(),
        settlement_correction_event_digest: event_digest.to_string(),
    }
}

pub(super) fn settlement_correction_posting_ref(
    posting_id: &str,
    posting_digest: &str,
) -> SettlementCorrectionPostingRef {
    SettlementCorrectionPostingRef {
        settlement_correction_posting_id: posting_id.to_string(),
        settlement_correction_posting_digest: posting_digest.to_string(),
    }
}
