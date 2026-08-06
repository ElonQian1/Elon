use anyhow::{bail, Result};

use super::{
    types::{
        AbortedComputePluginDownloadSegment, AuthorizedComputePluginDownloadSegment,
        CommittedComputePluginDownloadSegment, ComputePluginDownloadSegmentRequest,
        ComputePluginFetchAbortReason, ComputePluginFetchAuthoritySnapshot,
        DurablyWrittenComputePluginSegment, PreparedComputePluginFetchClaim,
        ValidatedComputePluginFetchAbortPermit, ValidatedComputePluginFetchCommitPermit,
    },
    ComputePluginFetchAuthorityPort,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission::{
        validate_inventory, validate_live_binding, validate_plan_window,
        AdmittedComputePluginDownload, AdmittedComputePluginInstallPlan,
    },
    install_plan_admission_validation::is_identifier,
    lifecycle::SLOT_DOWNLOADING,
    local_authority::{ComputePluginFetchAuthoritySession, ComputePluginPreparedFetchClaimFacts},
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

mod failure;

pub(super) use failure::{ComputePluginFetchAbortFailure, ComputePluginFetchAbortResult};
pub(in crate::node_agent_compute_plugin_host) use failure::{
    ComputePluginFetchCommitFailure, ComputePluginFetchCommitResult,
    ComputePluginFetchStoreMutationPhase,
};

/// Terminalizes a segment from one linear capability that owns the exact authorization, pinned
/// file and post-fsync trusted authority session. Every failure consumes authorization and leaves
/// only outcome recovery identity plus the still-open file.
pub(super) fn commit_download_segment(
    admitted: &AdmittedComputePluginInstallPlan,
    durable: DurablyWrittenComputePluginSegment<'_>,
) -> ComputePluginFetchCommitResult {
    let DurablyWrittenComputePluginSegment {
        authorized,
        mut file,
        root_lock_lease,
        resolution_session,
        sync_completed_at,
    } = durable;
    if let Err(error) = authorized
        .validate_recovery_session(resolution_session.authority_session())
        .and_then(|_| authorized.ensure_not_canceled())
        .and_then(|_| {
            validate_durable_binding(&authorized, &file, &resolution_session, sync_completed_at)
        })
    {
        return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreNotCalled,
            error,
            authorized.into_recovery_key(),
            file,
            root_lock_lease,
        ));
    }
    let download = match validate_authorized_binding(admitted, &authorized) {
        Ok(download) => download,
        Err(error) => {
            let recovery_key = authorized.into_recovery_key();
            return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
                ComputePluginFetchStoreMutationPhase::StoreNotCalled,
                error,
                recovery_key,
                file,
                root_lock_lease,
            ));
        }
    };
    let expected_end = match u64::try_from(authorized.claim.end_offset_bytes) {
        Ok(expected_end) => expected_end,
        Err(error) => {
            return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
                ComputePluginFetchStoreMutationPhase::StoreNotCalled,
                error.into(),
                authorized.into_recovery_key(),
                file,
                root_lock_lease,
            ));
        }
    };
    if let Err(error) = file.revalidate_exact_len(expected_end) {
        let recovery_key = authorized.into_recovery_key();
        return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreNotCalled,
            error,
            recovery_key,
            file,
            root_lock_lease,
        ));
    }
    let authority = ComputePluginFetchAuthorityPort {
        backend: resolution_session.authority_session(),
    };
    let request = ComputePluginDownloadSegmentRequest {
        ordinal: authorized.claim.ordinal,
        offset_bytes: authorized.claim.offset_bytes,
        length_bytes: authorized.claim.length_bytes,
        redirect_hop: authorized.redirect_hop,
        redirect_from_claim_id: (authorized.redirect_hop > 0)
            .then(|| authorized.claim.claim_id.clone()),
    };
    let snapshot = match authority.read_fresh_segment_authority(
        &authorized.claim.plan_id,
        &authorized.claim.plan_digest,
        download,
        &request,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let recovery_key = authorized.into_recovery_key();
            return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
                ComputePluginFetchStoreMutationPhase::StoreNotCalled,
                error,
                recovery_key,
                file,
                root_lock_lease,
            ));
        }
    };
    if let Err(error) = validate_commit_authority(admitted, download, &authorized.claim, &snapshot)
    {
        let recovery_key = authorized.into_recovery_key();
        return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreNotCalled,
            error,
            recovery_key,
            file,
            root_lock_lease,
        ));
    }
    let post_sync_trusted_at_ms = resolution_session.trusted_now_ms();
    if snapshot.store.trusted_now.timestamp_millis() != post_sync_trusted_at_ms
        || post_sync_trusted_at_ms <= authorized.claim.prepared_at_ms
        || post_sync_trusted_at_ms <= snapshot.store.observed_trusted_time_high_water_ms
    {
        let recovery_key = authorized.into_recovery_key();
        return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreNotCalled,
            anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_COMMIT_TIME_OBSERVATION_STALE"),
            recovery_key,
            file,
            root_lock_lease,
        ));
    }
    if let Err(error) = file
        .revalidate_exact_len(expected_end)
        .and_then(|_| authorized.ensure_not_canceled())
    {
        return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreNotCalled,
            error,
            authorized.into_recovery_key(),
            file,
            root_lock_lease,
        ));
    }
    let permit = ValidatedComputePluginFetchCommitPermit::new(
        &authorized.claim,
        &snapshot,
        file.identity_digest(),
    );
    if let Err(error) = authority.commit_validated_segment(permit) {
        let recovery_key = authorized.into_recovery_key();
        return Err(ComputePluginFetchCommitFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreOutcomeUncertain,
            error,
            recovery_key,
            file,
            root_lock_lease,
        ));
    }
    let committed = CommittedComputePluginDownloadSegment {
        ordinal: authorized.claim.ordinal,
        committed_offset: authorized.claim.end_offset_bytes,
        artifact_complete: authorized.claim.end_offset_bytes == download.download.size_bytes,
    };
    // Close the exact artifact handle before releasing the last possible lease on the root lock.
    // Pattern-bound locals otherwise have a different drop order from the custody structs.
    drop(file);
    drop(root_lock_lease);
    Ok(committed)
}

/// Consumes the network authorization without advancing the durable byte cursor. Only a fixed
/// internal reason crosses into SQLite; transport error text remains outside authority state.
pub(super) fn abort_download_segment<'authority>(
    admitted: &AdmittedComputePluginInstallPlan,
    authorized: AuthorizedComputePluginDownloadSegment,
    reason: ComputePluginFetchAbortReason,
    authority_session: ComputePluginFetchAuthoritySession<'authority>,
) -> ComputePluginFetchAbortResult<'authority> {
    if reason.is_cursor_damage() {
        return Err(
            ComputePluginFetchAbortFailure::recovery_binding_unavailable(
                anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_DAMAGE_REASON_REQUIRES_EVIDENCE"),
                authorized,
                authority_session,
            ),
        );
    }
    if let Err(error) = authorized.validate_recovery_session(&authority_session) {
        return Err(
            ComputePluginFetchAbortFailure::recovery_binding_unavailable(
                error,
                authorized,
                authority_session,
            ),
        );
    }
    if let Err(error) = validate_authorized_binding(admitted, &authorized) {
        let recovery_key = authorized.into_recovery_key();
        return Err(ComputePluginFetchAbortFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreNotCalled,
            error,
            recovery_key,
            authority_session,
        ));
    }
    let permit = ValidatedComputePluginFetchAbortPermit::new(&authorized.claim, reason);
    let authority = ComputePluginFetchAuthorityPort {
        backend: &authority_session,
    };
    if let Err(error) = authority.abort_validated_segment(permit) {
        let recovery_key = authorized.into_recovery_key();
        return Err(ComputePluginFetchAbortFailure::outcome_recovery_required(
            ComputePluginFetchStoreMutationPhase::StoreOutcomeUncertain,
            error,
            recovery_key,
            authority_session,
        ));
    }
    Ok(AbortedComputePluginDownloadSegment {
        ordinal: authorized.claim.ordinal,
        committed_offset: authorized.claim.offset_bytes,
        reason,
    })
}

pub(super) fn validate_authorized_binding<'admitted>(
    admitted: &'admitted AdmittedComputePluginInstallPlan,
    authorized: &AuthorizedComputePluginDownloadSegment,
) -> Result<&'admitted AdmittedComputePluginDownload> {
    let download = admitted
        .downloads()
        .get(authorized.claim.ordinal)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RESOLVE_ORDINAL"))?;
    let expected_end = authorized
        .claim
        .offset_bytes
        .checked_add(authorized.claim.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RESOLVE_RANGE_OVERFLOW"))?;
    if download != &authorized.download
        || authorized.claim.plan_id != admitted.plan().plan_id
        || authorized.claim.plan_digest != admitted.plan_digest()
        || authorized.claim.ordinal != authorized.download.ordinal
        || authorized.offset_bytes != authorized.claim.offset_bytes
        || authorized.length_bytes != authorized.claim.length_bytes
        || i64::from(authorized.redirect_hop) != authorized.claim.redirect_generation
        || authorized.claim.end_offset_bytes != expected_end
        || authorized.claim.end_offset_bytes > download.download.size_bytes
        || authorized.claim.authority_epoch <= 0
        || authorized.claim.process_owner_epoch <= 0
        || authorized.claim.cursor_generation <= 0
        || authorized.claim.redirect_generation < 0
        || authorized.claim.redirect_generation > 5
        || authorized.claim.prepared_at_ms < 0
        || !is_sha256(&authorized.claim.candidate_token_digest)
        || !super::relative_fetch_path_is_valid(&authorized.claim.part_relative_path)
    {
        bail!("COMPUTE_PLUGIN_FETCH_RESOLVE_HANDLE_CHANGED");
    }
    Ok(download)
}

fn validate_durable_binding(
    authorized: &AuthorizedComputePluginDownloadSegment,
    file: &crate::node_agent_managed_fs::PinnedManagedFile,
    resolution_session: &crate::node_agent_compute_plugin_host::local_authority::ComputePluginPostSyncFetchAuthoritySession<'_>,
    sync_completed_at: std::time::Instant,
) -> Result<()> {
    let expected_len = u64::try_from(authorized.claim.end_offset_bytes)?;
    if !is_sha256(file.identity_digest())
        || file.len_bytes() != expected_len
        || !resolution_session.was_observed_strictly_after(sync_completed_at)
        || resolution_session.trusted_now_ms() <= authorized.claim.prepared_at_ms
    {
        bail!("COMPUTE_PLUGIN_FETCH_DURABLE_FILE_BINDING_CHANGED");
    }
    Ok(())
}

fn validate_commit_authority(
    admitted: &AdmittedComputePluginInstallPlan,
    download: &AdmittedComputePluginDownload,
    claim: &PreparedComputePluginFetchClaim,
    snapshot: &ComputePluginFetchAuthoritySnapshot,
) -> Result<()> {
    let facts = &snapshot.store;
    validate_plan_window(admitted.plan(), facts.trusted_now.clone(), false)?;
    validate_live_binding(admitted.plan(), &facts.live)?;
    validate_inventory(&facts.inventory, facts.trusted_now.clone())?;
    let expected_application_revision = admitted
        .plan()
        .expected_inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_INVENTORY_REVISION_OVERFLOW"))?;
    let expected_grant_digest = admitted
        .plan()
        .items
        .get(download.item_index)
        .and_then(|item| item.grant.as_ref())
        .map(|grant| grant.grant_digest.as_str())
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_GRANT_BINDING_MISSING"))?;
    let prepared_matches = facts
        .prepared_claim
        .as_ref()
        .is_some_and(|prepared| prepared_claim_matches(claim, prepared));
    if facts.applied_plan_id != admitted.plan().plan_id
        || facts.applied_plan_digest != admitted.plan_digest()
        || facts.application_inventory_revision != expected_application_revision
        || facts.inventory.inventory_revision != facts.execution_inventory_revision
        || facts.inventory.inventory_revision < facts.application_inventory_revision
        || facts.authority_state_revision <= 0
        || facts.inventory_digest != jcs_sha256_hex(&facts.inventory)?
        || facts.planned_download != download.download
        || facts.candidate_release != download.release
        || facts.candidate_token_digest != claim.candidate_token_digest
        || facts.candidate_generation <= 0
        || facts.candidate_owner_plan_id != admitted.plan().plan_id
        || facts.candidate_owner_plan_digest != admitted.plan_digest()
        || facts.candidate_application_inventory_revision != expected_application_revision
        || facts.candidate_state != "owned"
        || facts.candidate_permission_grant_digest != expected_grant_digest
        || !is_identifier(&facts.slot_ref)
        || facts.slot_ref != format!("candidate_{}", facts.candidate_token_digest)
        || facts.part_relative_path != claim.part_relative_path
        || facts.authority_epoch != claim.authority_epoch
        || facts.process_owner_epoch != claim.process_owner_epoch
        || facts.download_cursor_generation != claim.cursor_generation
        || facts.committed_offset != claim.offset_bytes
        || facts.download_state != "downloading"
        || facts.download_updated_at_ms != claim.prepared_at_ms
        || facts.inventory.desired_policy_revision != admitted.plan().desired_policy_revision
        || facts.inventory.sharing_enabled != admitted.plan().sharing_enabled
        || !prepared_matches
    {
        bail!("COMPUTE_PLUGIN_FETCH_COMMIT_AUTHORITY_CHANGED");
    }
    let record = facts
        .inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == download.release.plugin_id)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_INVENTORY_MISSING"))?;
    let candidate_matches = record.candidate_slot_ref.as_ref().is_some_and(|slot_ref| {
        record.slots.iter().any(|slot| {
            &slot.slot_ref == slot_ref
                && slot.phase == SLOT_DOWNLOADING
                && slot.release == download.release
        })
    });
    if !candidate_matches
        || record.candidate_slot_ref.as_deref() != Some(facts.slot_ref.as_str())
        || facts.candidate_generation <= record.install_generation
    {
        bail!("COMPUTE_PLUGIN_FETCH_COMMIT_CANDIDATE_CHANGED");
    }
    Ok(())
}

fn prepared_claim_matches(
    expected: &PreparedComputePluginFetchClaim,
    actual: &ComputePluginPreparedFetchClaimFacts,
) -> bool {
    actual.claim_id == expected.claim_id
        && actual.plan_id == expected.plan_id
        && actual.plan_digest == expected.plan_digest
        && actual.ordinal == expected.ordinal
        && actual.candidate_token_digest == expected.candidate_token_digest
        && actual.part_relative_path == expected.part_relative_path
        && actual.authority_epoch == expected.authority_epoch
        && actual.process_owner_epoch == expected.process_owner_epoch
        && actual.cursor_generation == expected.cursor_generation
        && actual.redirect_generation == expected.redirect_generation
        && actual.offset_bytes == expected.offset_bytes
        && actual.length_bytes == expected.length_bytes
        && actual.end_offset_bytes == expected.end_offset_bytes
        && actual.prepared_at_ms == expected.prepared_at_ms
}
