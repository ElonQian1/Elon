//! Borrow-only runtime binding for retained launch-path candidates.
//!
//! This stage returns the exact admitted owner on both branches. It does not select a working
//! directory, acquire a component grant, seal loader authority, or start a process.

#![allow(dead_code)]

mod exact_context_plan;
mod prelease_pe_material;

use std::fmt;

use anyhow::{anyhow, bail, Error, Result};
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    discover_loader_launch_path_candidates, ManagedLoaderLaunchPathDiscoveryReceipt,
    ManagedLoaderLaunchPathDiscoverySet,
};

use super::super::work_admission_contract::DurableWorkAdmittedPluginSlot;

pub(in crate::node_agent_compute_plugin_host) use exact_context_plan::WindowsRunnerLaunchContextPreCreateProjection;
pub(super) use exact_context_plan::{
    consume_query_verified_loader_prerequisite, AuthenticatedWindowsRunnerLaunchContextIntent,
    PreliminaryResolutionRequestsPlannedWork, PreliminaryWindowsRunnerResolutionRequestPlanView,
    PreliminaryWindowsRunnerSelectedContextView, QueryVerifiedWindowsRunnerLaunchLineage,
    QueryVerifiedWindowsRunnerLaunchLineageValidationFailure,
    WindowsPreliminaryContentLeaseRequestRef, WindowsPreliminaryImportEdgeKind,
    WindowsPreliminaryLaunchPathComponentRequest, WindowsPreliminaryModuleEdgeLocator,
    WindowsPreliminaryModuleResolutionRequest, WindowsPreliminaryRetainedDirectoryLocation,
    WindowsPreliminarySearchDirectoryBinding, WindowsPreliminarySearchDirectoryTarget,
};
pub(super) use prelease_pe_material::symbol_is_exact;

const CANDIDATE_BINDING_DOMAIN: &[u8] = b"ELON_WINDOWS_RUNNER_LAUNCH_PATH_CANDIDATES_V1";

pub(super) struct WindowsRunnerLaunchPathCandidateSet {
    managed: ManagedLoaderLaunchPathDiscoverySet,
    runner_file_ordinal: usize,
    binding_digest: String,
}

#[must_use = "launch-path discovered admission custody must advance or be retained"]
pub(super) struct LaunchPathDiscoveredWork<'root> {
    admitted: DurableWorkAdmittedPluginSlot<'root>,
    candidates: WindowsRunnerLaunchPathCandidateSet,
}

#[must_use = "failed launch-path discovery returns the exact admitted owner"]
pub(super) struct LaunchPathDiscoveryFailure<'root> {
    error: Error,
    admitted: DurableWorkAdmittedPluginSlot<'root>,
}

/// Discovers application, package-root and every plan-directory candidate from retained handles.
/// The returned set deliberately has no selected-CWD field.
pub(super) fn discover_windows_runner_launch_path_candidates<'root>(
    admitted: DurableWorkAdmittedPluginSlot<'root>,
) -> std::result::Result<LaunchPathDiscoveredWork<'root>, LaunchPathDiscoveryFailure<'root>> {
    match bind_candidates(&admitted) {
        Ok(candidates) => Ok(LaunchPathDiscoveredWork {
            admitted,
            candidates,
        }),
        Err(error) => Err(LaunchPathDiscoveryFailure { error, admitted }),
    }
}

fn bind_candidates(
    admitted: &DurableWorkAdmittedPluginSlot<'_>,
) -> Result<WindowsRunnerLaunchPathCandidateSet> {
    let receipts = admitted.receipts();
    receipts.validate()?;
    let profile = receipts.source().source().launch_profile();
    let archive = admitted.installed().revalidated().staged().archive();
    let view = archive.launch_path_discovery_view();
    let envelope = view.plan().envelope();
    let plan = &envelope.plan;
    let evidence = view.evidence();
    let extracted = &evidence.evidence;

    if plan.files.len() != view.files().len()
        || plan.files.len() != extracted.files.len()
        || plan.directories.len() != view.directories().len()
        || extracted.extracted_file_count != i64::try_from(extracted.files.len())?
        || extracted.extraction_plan_digest != envelope.plan_digest
    {
        bail!("COMPUTE_PLUGIN_LAUNCH_PATH_DISCOVERY_CUSTODY_CHANGED");
    }

    for ((planned, observed), retained) in plan.files.iter().zip(&extracted.files).zip(view.files())
    {
        if planned.relative_path != observed.relative_path
            || planned.expected_digest != observed.digest
            || planned.expected_size_bytes != observed.size_bytes
            || retained.len_bytes() != u64::try_from(observed.size_bytes)?
            || retained.identity_digest() != observed.file_identity_digest
        {
            bail!("COMPUTE_PLUGIN_LAUNCH_PATH_DISCOVERY_FILE_SET_CHANGED");
        }
    }

    let mut runner_ordinals = plan
        .files
        .iter()
        .enumerate()
        .filter(|(_, file)| file.relative_path == profile.runner_relative_path())
        .map(|(ordinal, _)| ordinal);
    let runner_file_ordinal = runner_ordinals
        .next()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_LAUNCH_PATH_DISCOVERY_RUNNER_MISSING"))?;
    if runner_ordinals.next().is_some() {
        bail!("COMPUTE_PLUGIN_LAUNCH_PATH_DISCOVERY_RUNNER_DUPLICATED");
    }
    let planned_runner = &plan.files[runner_file_ordinal];
    let observed_runner = &extracted.files[runner_file_ordinal];
    let retained_runner = &view.files()[runner_file_ordinal];
    if !planned_runner.executable
        || planned_runner.expected_digest != profile.runner_file_digest()
        || planned_runner.expected_size_bytes != profile.runner_file_size_bytes()
        || observed_runner.relative_path != profile.runner_relative_path()
        || observed_runner.digest != profile.runner_file_digest()
        || observed_runner.size_bytes != profile.runner_file_size_bytes()
    {
        bail!("COMPUTE_PLUGIN_LAUNCH_PATH_DISCOVERY_RUNNER_CHANGED");
    }

    let managed = discover_loader_launch_path_candidates(
        retained_runner,
        view.package_root(),
        view.directories(),
    )?;
    let (managed_digest, directory_count) = managed.binding();
    let application_root = managed.application().binding().0;
    let package_component_count = managed.package_root().components().len();
    if directory_count != plan.directories.len()
        || application_root != extracted.root_identity_digest
        || managed.package_root().binding().0 != extracted.root_identity_digest
        || !receipt_matches_relative_path(
            managed.application(),
            package_component_count,
            profile.runner_relative_path(),
        )
        || managed
            .plan_directories()
            .iter()
            .enumerate()
            .any(|(ordinal, candidate)| {
                let (candidate_ordinal, receipt) = candidate.binding();
                candidate_ordinal != ordinal
                    || !receipt_matches_relative_path(
                        receipt,
                        package_component_count,
                        &plan.directories[ordinal],
                    )
            })
    {
        bail!("COMPUTE_PLUGIN_LAUNCH_PATH_DISCOVERY_MANAGED_SET_CHANGED");
    }

    let source_digest = receipts.source().source_digest();
    let receipt_digest = receipts.receipt().receipt_digest();
    let mut digest = BindingDigest::new(CANDIDATE_BINDING_DOMAIN);
    for value in [
        source_digest,
        receipt_digest,
        &envelope.plan_digest,
        &evidence.evidence_digest,
        retained_runner.identity_digest(),
        managed_digest,
    ] {
        digest.text(value);
    }
    digest.usize(runner_file_ordinal);
    Ok(WindowsRunnerLaunchPathCandidateSet {
        managed,
        runner_file_ordinal,
        binding_digest: digest.finish(),
    })
}

fn receipt_matches_relative_path(
    receipt: &ManagedLoaderLaunchPathDiscoveryReceipt,
    package_component_count: usize,
    relative_path: &str,
) -> bool {
    let expected = relative_path.split('/').collect::<Vec<_>>();
    let Some(observed) = receipt.components().get(package_component_count..) else {
        return false;
    };
    observed.len() == expected.len()
        && observed
            .iter()
            .zip(expected)
            .all(|(component, expected)| component.binding().2 == expected)
}

impl WindowsRunnerLaunchPathCandidateSet {
    pub(super) fn binding(&self) -> (&str, usize, &ManagedLoaderLaunchPathDiscoverySet) {
        (
            &self.binding_digest,
            self.runner_file_ordinal,
            &self.managed,
        )
    }
}

impl<'root> LaunchPathDiscoveryFailure<'root> {
    pub(super) fn into_parts(self) -> (Error, DurableWorkAdmittedPluginSlot<'root>) {
        (self.error, self.admitted)
    }
}

impl fmt::Debug for WindowsRunnerLaunchPathCandidateSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsRunnerLaunchPathCandidateSet")
            .field("runner_file_ordinal", &self.runner_file_ordinal)
            .field("managed", &"<retained-handle-observations>")
            .field("binding", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for LaunchPathDiscoveredWork<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LaunchPathDiscoveredWork")
            .field("admitted", &"<retained>")
            .field("candidates", &self.candidates)
            .finish()
    }
}

impl fmt::Debug for LaunchPathDiscoveryFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LaunchPathDiscoveryFailure")
            .field("admitted", &"<retained>")
            .finish()
    }
}

struct BindingDigest(Sha256);

impl BindingDigest {
    fn new(domain: &[u8]) -> Self {
        let mut digest = Sha256::new();
        digest.update(domain);
        Self(digest)
    }

    fn text(&mut self, value: &str) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value.as_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.0.update((value as u64).to_le_bytes());
    }

    fn finish(self) -> String {
        hex::encode(self.0.finalize())
    }
}
