//! By-value typestate for one recursive request -> grant -> candidate -> lease -> parse wave.

use std::convert::Infallible;

use anyhow::{bail, Result};

use crate::node_agent_managed_fs::{
    ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
    PinnedWindowsLoaderResolvedSystemImageCandidate,
};

use super::super::super::super::launch_path_discovery::AuthenticatedWindowsRecursiveResolutionPolicy;
use super::super::super::{
    PreFinalWindowsLoaderNamespaceGrantSet, WindowsLoaderPackageContentLeaseCustody,
    WindowsLoaderSearchedNameFenceCustody,
};
use super::super::{
    WindowsPostLeaseSystemImageParseReceipt, WindowsRecursiveWaveAcquisitionReceipt,
};
use super::{AuthenticatedWindowsRecursiveWaveResolutionPlan, WindowsRecursiveWaveRequestPlan};

/// Exact earlier graph, authenticated policy and live owners retained across every wave.
///
/// Package leases and filesystem positive outcomes each occur once in this owner. Later wave
/// records refer to already-retained package/preloaded/KnownDLL owners by typed ordinal instead of
/// cloning a handle or inventing a filesystem candidate for them.
#[must_use = "whole recursive graph must remain in the active wave or failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveResolutionAccumulatedCustody<
    'root,
> {
    root_namespace: PreFinalWindowsLoaderNamespaceGrantSet<'root>,
    authenticated_policy: AuthenticatedWindowsRecursiveResolutionPolicy,
    base_package_content_leases: Vec<WindowsLoaderPackageContentLeaseCustody>,
    retained_filesystem_content_leases:
        Vec<ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody>,
    completed_parse_receipts: Vec<WindowsPostLeaseSystemImageParseReceipt>,
    completed_acquisition_receipts: Vec<WindowsRecursiveWaveAcquisitionReceipt>,
    whole_state_digest: String,
}

struct WindowsRecursivePendingSearchedNameGrantRef {
    searched_name_ordinal: usize,
    search_directory_ordinal: usize,
    grant_request_digest: String,
}

struct WindowsRecursivePendingFilesystemCandidateRef {
    resolution_request_ordinal: usize,
    candidate_plan_digest: String,
}

struct WindowsRecursivePendingFilesystemLeaseRef {
    resolution_request_ordinal: usize,
    candidate_binding_digest: String,
    lease_request_digest: String,
}

struct WindowsRecursivePendingSameOwnerParseRef {
    parse_receipt_ordinal: usize,
    producer_module_request_ordinal: usize,
    source_owner_binding_digest: String,
}

/// Retained package lease is centralized in accumulated custody; this value is only its exact
/// typed parse reference.
struct WindowsRecursivePackageContentLeaseSourceRef {
    package_file_ordinal: usize,
    package_content_lease_binding_digest: String,
}

/// Exact authenticated bootstrap-section source; no filesystem candidate is involved.
struct WindowsRecursiveAuthenticatedPreloadedSourceRef {
    preloaded_module_ordinal: usize,
    component_identity_digest: String,
    immutable_section_identity_digest: String,
    authenticated_evidence_digest: String,
}

/// Exact KnownDLL section/mapping authority; no parent-relative file candidate is involved.
struct WindowsRecursiveKnownDllSectionSourceRef {
    known_dll_authority_record_ordinal: usize,
    section_identity_digest: String,
    immutable_section_identity_digest: String,
    section_image_mapping_receipt_digest: String,
}

/// Only the ordinary filesystem route owns a retained parent-relative candidate.
#[must_use = "filesystem candidate must enter one lease attempt or failure custody"]
struct WindowsRecursiveFilesystemCandidateCustody {
    resolution_request_ordinal: usize,
    candidate: PinnedWindowsLoaderResolvedSystemImageCandidate,
}

/// Positive lease response and same retained image owner used by the PE parser.
#[must_use = "filesystem lease outcome must remain through same-owner parse and final custody"]
struct WindowsRecursiveFilesystemLeaseSourceCustody {
    resolution_request_ordinal: usize,
    outcome: ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
}

/// Route-specific post-resolution owner. Three routes reuse typed immutable authority; only the
/// filesystem variant carries a candidate file into lease acquisition.
enum WindowsRecursiveRouteCandidateCustody {
    PackageContentLease(WindowsRecursivePackageContentLeaseSourceRef),
    AuthenticatedPreloadedModule(WindowsRecursiveAuthenticatedPreloadedSourceRef),
    KnownDllSection(WindowsRecursiveKnownDllSectionSourceRef),
    ResolvedFilesystemSystemImage(WindowsRecursiveFilesystemCandidateCustody),
}

/// Exact immutable owner from which one target image must be parsed. The enum prevents package,
/// preloaded or KnownDLL targets from being coerced through the filesystem lease route.
enum WindowsRecursiveSameOwnerParseSourceCustody {
    PackageContentLease(WindowsRecursivePackageContentLeaseSourceRef),
    AuthenticatedPreloadedModule(WindowsRecursiveAuthenticatedPreloadedSourceRef),
    KnownDllSection(WindowsRecursiveKnownDllSectionSourceRef),
    ResolvedFilesystemSystemImage(WindowsRecursiveFilesystemLeaseSourceCustody),
}

/// Whole graph plus canonical outgoing request plan, before the first grant dispatch.
#[must_use = "wave request custody must move whole into grant acquisition or remain intact"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveRequestCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    request_plan: WindowsRecursiveWaveRequestPlan,
    _wave_request_dispatch_producer_unavailable: Infallible,
}

/// Partial searched-name grant acquisition. Completed grants, active root and pending refs remain
/// together; a future dispatcher must consume this whole value for each next attempt.
#[must_use = "partial wave grants must advance whole or enter grant failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveGrantAcquisitionCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    request_plan: WindowsRecursiveWaveRequestPlan,
    resolved_plan: AuthenticatedWindowsRecursiveWaveResolutionPlan,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    pending_searched_name_grants: Vec<WindowsRecursivePendingSearchedNameGrantRef>,
    searched_name_grant_set_digest: String,
    _wave_grant_advancer_unavailable: Infallible,
}

impl WindowsRecursiveWaveGrantAcquisitionCustody<'_> {
    /// Purpose-specific borrow gate that must succeed before the first searched-name dispatch.
    /// The cumulative module/name/system totals are derived from exact contiguous request ranges;
    /// the authenticated resolver supplies parsed-image/frontier/depth projections that the final
    /// acquisition/closure validators must later cross-bind to actual receipts and edges.
    pub(super) fn validate_policy_limits_before_first_dispatch(&self) -> Result<()> {
        if self.resolved_plan.producer_wave_ordinal != self.request_plan.producer_wave_ordinal
            || self.resolved_plan.source_request_plan_digest
                != self.request_plan.source_request_plan_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PRE_DISPATCH_PLAN_CHANGED");
        }
        let projected_recursive_wave_count = self
            .request_plan
            .producer_wave_ordinal
            .checked_add(
                if self
                    .resolved_plan
                    .projected_next_frontier_parse_receipt_count
                    > 0
                {
                    1
                } else {
                    0
                },
            )
            .ok_or_else(projected_count_overflow)?;
        let projected_module_request_count = self
            .request_plan
            .first_module_request_ordinal
            .checked_add(self.request_plan.module_request_count)
            .ok_or_else(projected_count_overflow)?;
        let projected_searched_name_count = self
            .request_plan
            .first_searched_name_ordinal
            .checked_add(self.request_plan.searched_name_count)
            .ok_or_else(projected_count_overflow)?;
        let projected_system_image_request_count = self
            .request_plan
            .first_system_image_request_ordinal
            .checked_add(self.request_plan.system_image_request_count)
            .ok_or_else(projected_count_overflow)?;
        self.accumulated
            .authenticated_policy
            .validate_projected_totals_before_dispatch(
                projected_recursive_wave_count,
                self.resolved_plan.projected_parsed_image_count,
                projected_module_request_count,
                projected_searched_name_count,
                projected_system_image_request_count,
                self.resolved_plan.projected_forwarder_hop_depth,
            )
    }
}

fn projected_count_overflow() -> anyhow::Error {
    anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PRE_DISPATCH_COUNT_OVERFLOW")
}

/// Route selection after all required searched-name grants. Filesystem candidates first become
/// real owners in this post-grant state; other terminal routes retain purpose-specific refs.
#[must_use = "route candidates must advance whole into required leases or failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveCandidateAcquisitionCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    request_plan: WindowsRecursiveWaveRequestPlan,
    resolved_plan: AuthenticatedWindowsRecursiveWaveResolutionPlan,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    acquired_route_candidates: Vec<WindowsRecursiveRouteCandidateCustody>,
    pending_filesystem_candidates: Vec<WindowsRecursivePendingFilesystemCandidateRef>,
    filesystem_candidate_set_digest: String,
    _wave_candidate_advancer_unavailable: Infallible,
}

/// Lease acquisition owns every acquired non-filesystem source and every still-linear filesystem
/// candidate. An active lease attempt moves one candidate out and must be parked in failure
/// custody if no valid positive transition is available.
#[must_use = "partial wave leases must advance whole or enter lease failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveLeaseAcquisitionCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    request_plan: WindowsRecursiveWaveRequestPlan,
    resolved_plan: AuthenticatedWindowsRecursiveWaveResolutionPlan,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    acquired_parse_sources: Vec<WindowsRecursiveSameOwnerParseSourceCustody>,
    pending_filesystem_candidates: Vec<WindowsRecursiveFilesystemCandidateCustody>,
    pending_filesystem_leases: Vec<WindowsRecursivePendingFilesystemLeaseRef>,
    immutable_content_lease_set_digest: String,
    _wave_lease_advancer_unavailable: Infallible,
}

/// Same-owner parse stage. Every completed receipt remains beside its exact immutable source;
/// pending sources cannot be reduced to a path, digest or retry scalar.
#[must_use = "same-owner parse custody must complete whole or enter parse failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveSameOwnerParseCustody<
    'root,
> {
    accumulated: WindowsRecursiveResolutionAccumulatedCustody<'root>,
    request_plan: WindowsRecursiveWaveRequestPlan,
    resolved_plan: AuthenticatedWindowsRecursiveWaveResolutionPlan,
    acquired_searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    completed_parse_sources: Vec<WindowsRecursiveSameOwnerParseSourceCustody>,
    completed_parse_receipts: Vec<WindowsPostLeaseSystemImageParseReceipt>,
    pending_parse_sources: Vec<WindowsRecursiveSameOwnerParseSourceCustody>,
    pending_parse_refs: Vec<WindowsRecursivePendingSameOwnerParseRef>,
    same_owner_parse_set_digest: String,
    _same_owner_parser_advancer_unavailable: Infallible,
}

/// One completed producer acquisition, still retaining the exact sources and whole predecessor
/// graph. The next wave may be formed only by a future consuming sealer.
#[must_use = "completed wave must enter the next wave or terminal whole-chain custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursiveWaveCompletedCustody<
    'root,
> {
    parsed_wave: WindowsRecursiveWaveSameOwnerParseCustody<'root>,
    acquisition_receipt: WindowsRecursiveWaveAcquisitionReceipt,
    _completed_wave_sealer_unavailable: Infallible,
}

/// A base-only closure reaches the terminal state directly after `A0`; otherwise the last
/// recursive completed wave owns `AN`. This explicit sum avoids manufacturing an empty recursive
/// wave merely to represent the zero-wave case.
enum WindowsRecursiveTerminalPredecessorCustody<'root> {
    BaseOnly(WindowsRecursiveResolutionAccumulatedCustody<'root>),
    CompletedRecursiveWave(WindowsRecursiveWaveCompletedCustody<'root>),
}

/// Terminal `AN` custody. It is reachable only with an empty next frontier and retains either the
/// base-only accumulated graph or the final completed recursive wave by value.
#[must_use = "terminal recursive custody must move whole into the final closure sealer"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct TerminalWindowsRecursiveResolutionCustody<
    'root,
> {
    predecessor: WindowsRecursiveTerminalPredecessorCustody<'root>,
    terminal_empty_frontier_receipt_digest: String,
    _terminal_recursive_resolution_sealer_unavailable: Infallible,
}
