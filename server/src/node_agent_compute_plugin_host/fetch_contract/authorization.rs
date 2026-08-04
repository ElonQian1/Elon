use anyhow::{bail, Result};

use super::{
    authorization_failure::{
        ComputePluginFetchAuthorizationFailure, ComputePluginFetchAuthorizationResult,
        ComputePluginFetchRedirectFailure, ComputePluginFetchRedirectResult,
    },
    recovery,
    types::{
        AuthorizedComputePluginDownloadSegment, ComputePluginDownloadSegmentRequest,
        ComputePluginFetchAuthoritySnapshot, PreparedComputePluginFetchClaim,
        ValidatedComputePluginFetchClaimPermit,
    },
    validate_download_segment_authority, ComputePluginFetchAuthorityPort,
    MAX_DOWNLOAD_SEGMENT_BYTES, MAX_REDIRECT_HOPS,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::{AdmittedComputePluginDownload, AdmittedComputePluginInstallPlan},
    install_plan_admission_validation::is_identifier,
    local_authority::ComputePluginFetchAuthoritySession,
};

/// Call immediately before every request, redirect and resumed byte range. The authority owns the
/// durable cursor claim; callers cannot authorize from DTOs retained after initial admission.
pub(in crate::node_agent_compute_plugin_host) fn authorize_download_segment(
    admitted: &AdmittedComputePluginInstallPlan,
    request: &ComputePluginDownloadSegmentRequest,
    authority_session: &ComputePluginFetchAuthoritySession<'_>,
) -> ComputePluginFetchAuthorizationResult {
    let authority = ComputePluginFetchAuthorityPort {
        backend: authority_session,
    };
    let plan = admitted.plan();
    let download = admitted
        .downloads()
        .get(request.ordinal)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_ORDINAL: download is not in plan"))
        .map_err(ComputePluginFetchAuthorizationFailure::rejected)?;
    let segment_end = request
        .offset_bytes
        .checked_add(request.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RANGE_OVERFLOW"))
        .map_err(ComputePluginFetchAuthorizationFailure::rejected)?;
    if download.ordinal != request.ordinal
        || request.offset_bytes < 0
        || request.length_bytes <= 0
        || request.length_bytes > MAX_DOWNLOAD_SEGMENT_BYTES
        || segment_end > download.download.size_bytes
        || request.redirect_hop > MAX_REDIRECT_HOPS
        || (request.redirect_hop == 0) != request.redirect_from_claim_id.is_none()
        || request
            .redirect_from_claim_id
            .as_deref()
            .is_some_and(|claim_id| !is_identifier(claim_id))
    {
        return Err(ComputePluginFetchAuthorizationFailure::rejected(
            anyhow::anyhow!(
                "COMPUTE_PLUGIN_FETCH_RANGE: segment or redirect hop is outside the plan"
            ),
        ));
    }
    let facts = authority
        .read_fresh_segment_authority(&plan.plan_id, admitted.plan_digest(), download, request)
        .map_err(ComputePluginFetchAuthorizationFailure::rejected)?;
    validate_download_segment_authority(admitted, download, request, &facts)
        .map_err(ComputePluginFetchAuthorizationFailure::rejected)?;
    let claim_id = request
        .redirect_from_claim_id
        .clone()
        .unwrap_or_else(|| format!("fetch_{}", uuid::Uuid::new_v4().simple()));
    let expected_claim = expected_prepared_fetch_claim(admitted, request, &facts, claim_id)
        .map_err(ComputePluginFetchAuthorizationFailure::rejected)?;
    let observed_redirect_generation = facts
        .store
        .prepared_claim
        .as_ref()
        .map(|claim| claim.redirect_generation)
        .unwrap_or(0);
    let recovery_key = recovery::capture_expected_claim_recovery_key(
        download,
        &expected_claim,
        observed_redirect_generation,
        &facts,
        authority_session,
    );
    let permit = ValidatedComputePluginFetchClaimPermit::new(
        &plan.plan_id,
        admitted.plan_digest(),
        &expected_claim.claim_id,
        request,
        &facts,
    );
    let claim = match authority.claim_validated_segment(download, permit) {
        Ok(claim) => claim,
        Err(error) => {
            return Err(
                ComputePluginFetchAuthorizationFailure::outcome_recovery_required(
                    error,
                    recovery_key,
                ),
            );
        }
    };
    let recovery_key = recovery_key.into_claim_observed();
    if let Err(error) =
        validate_returned_fetch_claim(admitted, download, request, &facts, &expected_claim, &claim)
    {
        return Err(
            ComputePluginFetchAuthorizationFailure::outcome_recovery_required(error, recovery_key),
        );
    }
    Ok(AuthorizedComputePluginDownloadSegment {
        download: download.clone(),
        offset_bytes: request.offset_bytes,
        length_bytes: request.length_bytes,
        redirect_hop: request.redirect_hop,
        claim,
        recovery_key,
    })
}

/// Redirect returns the complete old authorization when Store was not called. Once Store is
/// called, only the claim recovery identity survives, so an uncertain redirect cannot be retried.
pub(in crate::node_agent_compute_plugin_host) fn authorize_download_redirect(
    admitted: &AdmittedComputePluginInstallPlan,
    authorized: AuthorizedComputePluginDownloadSegment,
    authority: &ComputePluginFetchAuthoritySession<'_>,
) -> ComputePluginFetchRedirectResult {
    if let Err(error) = authorized.validate_recovery_session(authority) {
        return Err(ComputePluginFetchRedirectFailure::store_not_called(
            error, authorized,
        ));
    }
    let redirect_hop = match authorized
        .redirect_hop
        .checked_add(1)
        .filter(|hop| *hop <= MAX_REDIRECT_HOPS)
    {
        Some(hop) => hop,
        None => {
            return Err(ComputePluginFetchRedirectFailure::store_not_called(
                anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_REDIRECT_LIMIT"),
                authorized,
            ));
        }
    };
    let request = ComputePluginDownloadSegmentRequest {
        ordinal: authorized.download.ordinal,
        offset_bytes: authorized.offset_bytes,
        length_bytes: authorized.length_bytes,
        redirect_hop,
        redirect_from_claim_id: Some(authorized.claim.claim_id.clone()),
    };
    match authorize_download_segment(admitted, &request, authority) {
        Ok(next) => Ok(next),
        Err(ComputePluginFetchAuthorizationFailure::RejectedBeforeClaim { error }) => Err(
            ComputePluginFetchRedirectFailure::store_not_called(error, authorized),
        ),
        Err(ComputePluginFetchAuthorizationFailure::ClaimOutcomeRecoveryRequired {
            error,
            recovery_key,
        }) => {
            Err(ComputePluginFetchRedirectFailure::outcome_recovery_required(error, recovery_key))
        }
    }
}

fn expected_prepared_fetch_claim(
    admitted: &AdmittedComputePluginInstallPlan,
    request: &ComputePluginDownloadSegmentRequest,
    snapshot: &ComputePluginFetchAuthoritySnapshot,
    claim_id: String,
) -> Result<PreparedComputePluginFetchClaim> {
    let facts = &snapshot.store;
    let end_offset_bytes = request
        .offset_bytes
        .checked_add(request.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RANGE_OVERFLOW"))?;
    let (cursor_generation, prepared_at_ms) = if request.redirect_hop == 0 {
        (
            facts
                .download_cursor_generation
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CURSOR_EXHAUSTED"))?,
            facts.trusted_now.timestamp_millis(),
        )
    } else {
        let prepared = facts
            .prepared_claim
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_REDIRECT_EXPECTATION_MISSING"))?;
        (facts.download_cursor_generation, prepared.prepared_at_ms)
    };
    if !is_identifier(&claim_id) {
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_ID_INVALID");
    }
    Ok(PreparedComputePluginFetchClaim {
        claim_id,
        plan_id: admitted.plan().plan_id.clone(),
        plan_digest: admitted.plan_digest().to_string(),
        ordinal: request.ordinal,
        candidate_token_digest: facts.candidate_token_digest.clone(),
        part_relative_path: facts.part_relative_path.clone(),
        authority_epoch: facts.authority_epoch,
        process_owner_epoch: facts.process_owner_epoch,
        cursor_generation,
        redirect_generation: i64::from(request.redirect_hop),
        offset_bytes: request.offset_bytes,
        length_bytes: request.length_bytes,
        end_offset_bytes,
        prepared_at_ms,
    })
}

fn validate_returned_fetch_claim(
    admitted: &AdmittedComputePluginInstallPlan,
    download: &AdmittedComputePluginDownload,
    request: &ComputePluginDownloadSegmentRequest,
    snapshot: &ComputePluginFetchAuthoritySnapshot,
    expected: &PreparedComputePluginFetchClaim,
    claim: &PreparedComputePluginFetchClaim,
) -> Result<()> {
    let facts = &snapshot.store;
    let expected_end = request
        .offset_bytes
        .checked_add(request.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RANGE_OVERFLOW"))?;
    let expected_cursor = if request.redirect_hop == 0 {
        facts
            .download_cursor_generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_CURSOR_EXHAUSTED"))?
    } else {
        facts.download_cursor_generation
    };
    let prepared_at_matches = if let Some(prepared) = facts.prepared_claim.as_ref() {
        claim.prepared_at_ms == prepared.prepared_at_ms
    } else {
        claim.prepared_at_ms == facts.trusted_now.timestamp_millis()
    };
    let redirect_claim_matches = if request.redirect_hop == 0 {
        facts.prepared_claim.is_none()
    } else {
        facts.prepared_claim.as_ref().is_some_and(|prepared| {
            claim.claim_id == prepared.claim_id
                && request.redirect_from_claim_id.as_deref() == Some(prepared.claim_id.as_str())
        })
    };
    if claim != expected
        || !is_identifier(&claim.claim_id)
        || claim.plan_id != admitted.plan().plan_id
        || claim.plan_digest != admitted.plan_digest()
        || claim.ordinal != request.ordinal
        || claim.ordinal != download.ordinal
        || claim.candidate_token_digest != facts.candidate_token_digest
        || claim.part_relative_path != facts.part_relative_path
        || claim.authority_epoch != facts.authority_epoch
        || claim.process_owner_epoch != facts.process_owner_epoch
        || claim.cursor_generation != expected_cursor
        || claim.redirect_generation != i64::from(request.redirect_hop)
        || claim.offset_bytes != request.offset_bytes
        || claim.length_bytes != request.length_bytes
        || claim.end_offset_bytes != expected_end
        || !prepared_at_matches
        || !redirect_claim_matches
    {
        bail!("COMPUTE_PLUGIN_FETCH_CLAIM_RETURN_BINDING_MISMATCH");
    }
    Ok(())
}
