use anyhow::{bail, Result};

use super::{
    install_plan::ComputePluginInstallPlan,
    install_plan_admission::{
        validate_inventory, validate_live_binding, validate_plan_window,
        AdmittedComputePluginDownload, AdmittedComputePluginInstallPlan,
    },
    install_plan_admission_validation::is_identifier,
    lifecycle::SLOT_DOWNLOADING,
    local_authority::{
        ComputePluginFetchAuthorityFacts, ComputePluginFetchAuthoritySession,
        ComputePluginPreparedFetchClaimFacts,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

// Kept private until outcome recovery and file-cursor reconciliation can close every consumed
// handle error without reusing a stale mutation capability.
mod recovery;
mod resolution;
mod types;

pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginFetchClaimOutcome, ComputePluginFetchClaimOutcomeKind,
    ComputePluginFetchClaimRecoveryKey, ValidatedComputePluginFetchRecoveryAbortPermit,
};
pub(crate) use types::{
    AuthorizedComputePluginDownloadSegment, ComputePluginDownloadSegmentRequest,
};
use types::{ComputePluginFetchAuthoritySnapshot, PreparedComputePluginFetchClaim};
pub(in crate::node_agent_compute_plugin_host) use types::{
    ValidatedComputePluginFetchAbortPermit, ValidatedComputePluginFetchClaimPermit,
    ValidatedComputePluginFetchCommitPermit,
};

const MAX_DOWNLOAD_SEGMENT_BYTES: i64 = 16 * 1_024 * 1_024;
const MAX_REDIRECT_HOPS: u8 = 5;

trait ComputePluginFetchAuthorityBackend {
    /// Performs a fresh side-effect-free authoritative read. A cached admission snapshot or wall
    /// clock without a persisted monotonic high-water is not valid. The sealed session holds one
    /// authenticated trusted-time observation for this read and its immediately following CAS.
    fn read_fresh_segment_authority(
        &self,
        plan_id: &str,
        plan_digest: &str,
        download: &AdmittedComputePluginDownload,
        request: &ComputePluginDownloadSegmentRequest,
    ) -> Result<ComputePluginFetchAuthoritySnapshot>;

    /// Atomically re-reads every field and fence in `validated`, then creates the durable claim
    /// only if they are unchanged. Redirects update the exact existing claim and preserve cursor.
    /// The concrete Store must validate its returned claim before committing the transaction;
    /// validation after this method returns is defense-in-depth, not a rollback mechanism.
    fn claim_validated_segment(
        &self,
        download: &AdmittedComputePluginDownload,
        permit: ValidatedComputePluginFetchClaimPermit<'_>,
    ) -> Result<PreparedComputePluginFetchClaim>;

    fn commit_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchCommitPermit<'_>,
    ) -> Result<()>;

    fn abort_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchAbortPermit<'_>,
    ) -> Result<()>;
}

/// Opaque access to the one concrete Store backend. Its constructor and backend implementation
/// live in this module once the Store lands, so crate callers cannot invoke read/CAS primitives.
struct ComputePluginFetchAuthorityPort<'authority> {
    backend: &'authority dyn ComputePluginFetchAuthorityBackend,
}

impl ComputePluginFetchAuthorityPort<'_> {
    fn read_fresh_segment_authority(
        &self,
        plan_id: &str,
        plan_digest: &str,
        download: &AdmittedComputePluginDownload,
        request: &ComputePluginDownloadSegmentRequest,
    ) -> Result<ComputePluginFetchAuthoritySnapshot> {
        self.backend
            .read_fresh_segment_authority(plan_id, plan_digest, download, request)
    }

    fn claim_validated_segment(
        &self,
        download: &AdmittedComputePluginDownload,
        permit: ValidatedComputePluginFetchClaimPermit<'_>,
    ) -> Result<PreparedComputePluginFetchClaim> {
        self.backend.claim_validated_segment(download, permit)
    }

    fn commit_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchCommitPermit<'_>,
    ) -> Result<()> {
        self.backend.commit_validated_segment(permit)
    }

    fn abort_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchAbortPermit<'_>,
    ) -> Result<()> {
        self.backend.abort_validated_segment(permit)
    }
}

impl ComputePluginFetchAuthorityBackend for ComputePluginFetchAuthoritySession<'_> {
    fn read_fresh_segment_authority(
        &self,
        plan_id: &str,
        plan_digest: &str,
        download: &AdmittedComputePluginDownload,
        request: &ComputePluginDownloadSegmentRequest,
    ) -> Result<ComputePluginFetchAuthoritySnapshot> {
        if download.ordinal != request.ordinal {
            bail!("COMPUTE_PLUGIN_FETCH_BACKEND_ORDINAL_CHANGED");
        }
        Ok(ComputePluginFetchAuthoritySnapshot {
            store: ComputePluginFetchAuthoritySession::read_fresh_segment_authority(
                self,
                plan_id,
                plan_digest,
                request.ordinal,
            )?,
        })
    }

    fn claim_validated_segment(
        &self,
        download: &AdmittedComputePluginDownload,
        permit: ValidatedComputePluginFetchClaimPermit<'_>,
    ) -> Result<PreparedComputePluginFetchClaim> {
        if download.ordinal != permit.ordinal() {
            bail!("COMPUTE_PLUGIN_FETCH_BACKEND_ORDINAL_CHANGED");
        }
        let prepared = ComputePluginFetchAuthoritySession::claim_validated_segment(self, permit)?;
        Ok(prepared.into())
    }

    fn commit_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchCommitPermit<'_>,
    ) -> Result<()> {
        ComputePluginFetchAuthoritySession::commit_validated_segment(self, permit)
    }

    fn abort_validated_segment(
        &self,
        permit: ValidatedComputePluginFetchAbortPermit<'_>,
    ) -> Result<()> {
        ComputePluginFetchAuthoritySession::abort_validated_segment(self, permit)
    }
}

impl From<ComputePluginPreparedFetchClaimFacts> for PreparedComputePluginFetchClaim {
    fn from(claim: ComputePluginPreparedFetchClaimFacts) -> Self {
        Self {
            claim_id: claim.claim_id,
            plan_id: claim.plan_id,
            plan_digest: claim.plan_digest,
            ordinal: claim.ordinal,
            candidate_token_digest: claim.candidate_token_digest,
            part_relative_path: claim.part_relative_path,
            authority_epoch: claim.authority_epoch,
            process_owner_epoch: claim.process_owner_epoch,
            cursor_generation: claim.cursor_generation,
            redirect_generation: claim.redirect_generation,
            offset_bytes: claim.offset_bytes,
            length_bytes: claim.length_bytes,
            end_offset_bytes: claim.end_offset_bytes,
            prepared_at_ms: claim.prepared_at_ms,
        }
    }
}

/// Call immediately before every request, redirect and resumed byte range. The authority owns the
/// durable cursor claim; callers cannot authorize from the DTOs retained after initial admission.
pub(in crate::node_agent_compute_plugin_host) fn authorize_download_segment(
    admitted: &AdmittedComputePluginInstallPlan,
    request: &ComputePluginDownloadSegmentRequest,
    authority_session: &ComputePluginFetchAuthoritySession<'_>,
) -> Result<AuthorizedComputePluginDownloadSegment> {
    let authority = ComputePluginFetchAuthorityPort {
        backend: authority_session,
    };
    let plan = admitted.plan();
    let download = admitted
        .downloads()
        .get(request.ordinal)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_ORDINAL: download is not in plan"))?;
    let segment_end = request
        .offset_bytes
        .checked_add(request.length_bytes)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RANGE_OVERFLOW"))?;
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
        bail!("COMPUTE_PLUGIN_FETCH_RANGE: segment or redirect hop is outside the plan");
    }
    let facts = authority.read_fresh_segment_authority(
        &plan.plan_id,
        admitted.plan_digest(),
        download,
        request,
    )?;
    validate_download_segment_authority(admitted, download, request, &facts)?;
    let permit = ValidatedComputePluginFetchClaimPermit::new(
        &plan.plan_id,
        admitted.plan_digest(),
        request,
        &facts,
    );
    let claim = authority.claim_validated_segment(download, permit)?;
    validate_returned_fetch_claim(admitted, download, request, &facts, &claim)?;
    Ok(AuthorizedComputePluginDownloadSegment {
        download: download.clone(),
        offset_bytes: request.offset_bytes,
        length_bytes: request.length_bytes,
        redirect_hop: request.redirect_hop,
        claim,
    })
}

/// Consumes the prior handle and returns its next redirect generation. Every error drops the handle
/// and enters authority recovery, so an uncertain Store outcome cannot leave a usable stale claim.
pub(in crate::node_agent_compute_plugin_host) fn authorize_download_redirect(
    admitted: &AdmittedComputePluginInstallPlan,
    authorized: AuthorizedComputePluginDownloadSegment,
    authority: &ComputePluginFetchAuthoritySession<'_>,
) -> Result<AuthorizedComputePluginDownloadSegment> {
    let redirect_hop = authorized
        .redirect_hop
        .checked_add(1)
        .filter(|hop| *hop <= MAX_REDIRECT_HOPS)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_REDIRECT_LIMIT"))?;
    let request = ComputePluginDownloadSegmentRequest {
        ordinal: authorized.download.ordinal,
        offset_bytes: authorized.offset_bytes,
        length_bytes: authorized.length_bytes,
        redirect_hop,
        redirect_from_claim_id: Some(authorized.claim.claim_id.clone()),
    };
    authorize_download_segment(admitted, &request, authority)
}

pub(super) fn validate_download_segment_authority(
    admitted: &AdmittedComputePluginInstallPlan,
    download: &AdmittedComputePluginDownload,
    request: &ComputePluginDownloadSegmentRequest,
    snapshot: &ComputePluginFetchAuthoritySnapshot,
) -> Result<()> {
    let facts = &snapshot.store;
    let plan = admitted.plan();
    validate_plan_window(plan, facts.trusted_now.clone(), false)?;
    validate_live_binding(plan, &facts.live)?;
    validate_inventory(&facts.inventory, facts.trusted_now.clone())?;
    let expected_application_revision = plan
        .expected_inventory_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_INVENTORY_REVISION_OVERFLOW"))?;
    let expected_grant_digest = plan
        .items
        .get(download.item_index)
        .and_then(|item| item.grant.as_ref())
        .map(|grant| grant.grant_digest.as_str())
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_GRANT_BINDING_MISSING"))?;
    let expected_part_relative_path = format!(
        "compute-plugin/candidates/{}/downloads/{:04}-{}.part",
        facts.candidate_token_digest, request.ordinal, download.download.digest
    );
    let download_state_matches_hop = if request.redirect_hop == 0 {
        matches!(
            facts.download_state.as_str(),
            "pending" | "downloading" | "failed"
        )
    } else {
        facts.download_cursor_generation > 0 && facts.download_state == "downloading"
    };
    if facts.applied_plan_id != plan.plan_id
        || facts.applied_plan_digest != admitted.plan_digest()
        || facts.application_inventory_revision != expected_application_revision
        || facts.inventory.inventory_revision != facts.execution_inventory_revision
        || facts.inventory.inventory_revision < facts.application_inventory_revision
        || facts.authority_state_revision <= 0
        || facts.inventory_digest != jcs_sha256_hex(&facts.inventory)?
        || facts.authority_epoch <= 0
        || facts.process_owner_epoch <= 0
        || !is_sha256(&facts.candidate_token_digest)
        || facts.candidate_generation <= 0
        || facts.candidate_owner_plan_id != plan.plan_id
        || facts.candidate_owner_plan_digest != admitted.plan_digest()
        || facts.candidate_application_inventory_revision != expected_application_revision
        || facts.candidate_state != "owned"
        || facts.candidate_release != download.release
        || facts.candidate_permission_grant_digest != expected_grant_digest
        || !is_identifier(&facts.slot_ref)
        || facts.slot_ref != format!("candidate_{}", facts.candidate_token_digest)
        || facts.planned_download != download.download
        || facts.part_relative_path != expected_part_relative_path
        || !relative_fetch_path_is_valid(&facts.part_relative_path)
        || facts.committed_offset != request.offset_bytes
        || facts.download_cursor_generation < 0
        || !download_state_matches_hop
        || facts.inventory.desired_policy_revision != plan.desired_policy_revision
        || facts.inventory.sharing_enabled != plan.sharing_enabled
    {
        bail!("COMPUTE_PLUGIN_FETCH_BINDING_CHANGED: applied plan or inventory has changed");
    }
    let record = facts
        .inventory
        .plugins
        .iter()
        .find(|record| record.plugin_id == download.release.plugin_id)
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_INVENTORY: plugin record is missing")
        })?;
    let candidate_matches = record.candidate_slot_ref.as_ref().is_some_and(|slot_ref| {
        record.slots.iter().any(|slot| {
            &slot.slot_ref == slot_ref
                && slot.phase == SLOT_DOWNLOADING
                && slot.release == download.release
        })
    });
    if !candidate_matches {
        bail!("COMPUTE_PLUGIN_FETCH_SLOT_CHANGED: candidate slot is no longer owned by this plan");
    }
    if record.candidate_slot_ref.as_deref() != Some(facts.slot_ref.as_str())
        || facts.candidate_generation <= record.install_generation
    {
        bail!("COMPUTE_PLUGIN_FETCH_CANDIDATE_FENCE_CHANGED");
    }
    validate_prepared_claim_lineage(plan, request, facts)
}

fn validate_prepared_claim_lineage(
    plan: &ComputePluginInstallPlan,
    request: &ComputePluginDownloadSegmentRequest,
    facts: &ComputePluginFetchAuthorityFacts,
) -> Result<()> {
    match (
        request.redirect_hop,
        request.redirect_from_claim_id.as_deref(),
        facts.prepared_claim.as_ref(),
    ) {
        (0, None, None) => Ok(()),
        (hop, Some(expected_claim_id), Some(prepared)) if hop > 0 => {
            let expected_end = request
                .offset_bytes
                .checked_add(request.length_bytes)
                .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_FETCH_RANGE_OVERFLOW"))?;
            if !is_identifier(expected_claim_id)
                || prepared.claim_id != expected_claim_id
                || prepared.plan_id != plan.plan_id
                || prepared.plan_digest != facts.applied_plan_digest
                || prepared.ordinal != request.ordinal
                || prepared.candidate_token_digest != facts.candidate_token_digest
                || prepared.part_relative_path != facts.part_relative_path
                || prepared.authority_epoch != facts.authority_epoch
                || prepared.process_owner_epoch != facts.process_owner_epoch
                || prepared.cursor_generation != facts.download_cursor_generation
                || prepared
                    .redirect_generation
                    .checked_add(1)
                    .is_none_or(|next| next != i64::from(hop))
                || prepared.offset_bytes != request.offset_bytes
                || prepared.length_bytes != request.length_bytes
                || prepared.end_offset_bytes != expected_end
                || prepared.prepared_at_ms < 0
                || prepared.prepared_at_ms > facts.trusted_now.timestamp_millis()
            {
                bail!("COMPUTE_PLUGIN_FETCH_REDIRECT_LINEAGE_CHANGED");
            }
            Ok(())
        }
        _ => bail!("COMPUTE_PLUGIN_FETCH_REDIRECT_LINEAGE_CHANGED"),
    }
}

fn validate_returned_fetch_claim(
    admitted: &AdmittedComputePluginInstallPlan,
    download: &AdmittedComputePluginDownload,
    request: &ComputePluginDownloadSegmentRequest,
    snapshot: &ComputePluginFetchAuthoritySnapshot,
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
    if !is_identifier(&claim.claim_id)
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

fn relative_fetch_path_is_valid(value: &str) -> bool {
    let path = std::path::Path::new(value);
    !value.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}
