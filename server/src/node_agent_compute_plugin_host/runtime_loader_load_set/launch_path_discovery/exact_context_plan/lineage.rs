//! Query-verified retained launch lineage and its borrow-only process projection.

use anyhow::{bail, Error, Result};

use crate::node_agent_compute_plugin_host::{
    runtime_loader_load_set::model::{
        SealedComputePluginRunnerImage, WindowsLoaderWorkingDirectoryLocation,
    },
    runtime_loader_load_set::resolution::{
        GrantReadyWindowsRunnerResolutionPlan, PostLeaseSplitWindowsRunnerLoadSetPrerequisite,
        SealedWindowsRunnerLoadSetPrerequisite, WindowsLoaderPackageContentLeaseCustody,
    },
    work_admission_contract::{
        ComputePluginWorkAdmissionLaunchProfile, DurableWorkAdmittedPluginSlot,
    },
};

use super::{
    binding, AuthenticatedWindowsRunnerLaunchContextIntent,
    PreliminaryResolutionRequestsPlannedWork, PreliminaryWindowsRunnerResolutionRequestPlan,
};
use crate::node_agent_compute_plugin_host::runtime_loader_load_set::launch_path_discovery::{
    prelease_pe_material::AuthenticatedWindowsPreLeasePeMaterial, LaunchPathDiscoveredWork,
    WindowsRunnerLaunchPathCandidateSet,
};

/// Borrow-only pre-create projection. It is not live process evidence, an authority, or a
/// substitute for the blocked post-create machine/WOW64 queryback receipt.
pub(in crate::node_agent_compute_plugin_host) struct WindowsRunnerLaunchContextPreCreateProjection<
    'actual,
> {
    pub(super) launch_context_selector_digest: &'actual str,
    pub(super) process_machine_context_digest: &'actual str,
    pub(super) startup_import_resolution_profile_digest: &'actual str,
    pub(super) working_directory_identity_digest: &'actual str,
    pub(super) runner_relative_path: &'actual str,
    pub(super) entrypoint_arguments_digest: &'actual str,
    pub(super) restricted_token: bool,
    pub(super) app_container: bool,
    pub(super) inherited_handles: bool,
    pub(super) environment_policy: &'static str,
    pub(super) process_creation_flags: &'static [&'static str],
}

impl<'actual> WindowsRunnerLaunchContextPreCreateProjection<'actual> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::node_agent_compute_plugin_host) fn new(
        launch_context_selector_digest: &'actual str,
        process_machine_context_digest: &'actual str,
        startup_import_resolution_profile_digest: &'actual str,
        working_directory_identity_digest: &'actual str,
        runner_relative_path: &'actual str,
        entrypoint_arguments_digest: &'actual str,
        restricted_token: bool,
        app_container: bool,
        inherited_handles: bool,
        environment_policy: &'static str,
        process_creation_flags: &'static [&'static str],
    ) -> Self {
        Self {
            launch_context_selector_digest,
            process_machine_context_digest,
            startup_import_resolution_profile_digest,
            working_directory_identity_digest,
            runner_relative_path,
            entrypoint_arguments_digest,
            restricted_token,
            app_container,
            inherited_handles,
            environment_policy,
            process_creation_flags,
        }
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn loader_binding(
        &self,
    ) -> (&str, &str, &str, &str, &str, &str) {
        (
            self.launch_context_selector_digest,
            self.process_machine_context_digest,
            self.startup_import_resolution_profile_digest,
            self.working_directory_identity_digest,
            self.runner_relative_path,
            self.entrypoint_arguments_digest,
        )
    }
}

/// Immutable authenticated lineage retained after the admitted owner is split. Discovery
/// receipts, selected-context binding, prelease PE material and both request plans remain together
/// through process custody; no success path may collapse them to detached digests.
#[must_use = "authenticated launch lineage must remain with loader/process successor custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct QueryVerifiedWindowsRunnerLaunchLineage
{
    context: AuthenticatedWindowsRunnerLaunchContextIntent,
    candidates: WindowsRunnerLaunchPathCandidateSet,
    pe_material: AuthenticatedWindowsPreLeasePeMaterial,
    plan: PreliminaryWindowsRunnerResolutionRequestPlan,
    grant_ready_plan: GrantReadyWindowsRunnerResolutionPlan,
}

#[must_use = "failed query-verified lineage validation retains the whole prerequisite"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct QueryVerifiedWindowsRunnerLaunchLineageValidationFailure<
    'root,
> {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) error: Error,
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) prerequisite:
        SealedWindowsRunnerLoadSetPrerequisite<'root>,
}

/// The only success seam that can recover the admitted owner consumes the whole query-verified
/// prerequisite. Preliminary request, grant-ready, grant, lease, or pre-query owners cannot call it.
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn consume_query_verified_loader_prerequisite<
    'root,
>(
    prerequisite: SealedWindowsRunnerLoadSetPrerequisite<'root>,
) -> std::result::Result<
    (
        DurableWorkAdmittedPluginSlot<'root>,
        QueryVerifiedWindowsRunnerLaunchLineage,
        PostLeaseSplitWindowsRunnerLoadSetPrerequisite,
        Vec<WindowsLoaderPackageContentLeaseCustody>,
    ),
    QueryVerifiedWindowsRunnerLaunchLineageValidationFailure<'root>,
> {
    if let Err(error) = validate_query_verified_lineage(&prerequisite) {
        return Err(QueryVerifiedWindowsRunnerLaunchLineageValidationFailure {
            error,
            prerequisite,
        });
    }
    let SealedWindowsRunnerLoadSetPrerequisite {
        postlease_lineage,
        namespace,
        resolution,
        package_content_leases,
    } = prerequisite;
    let (preliminary, grant_ready_plan) = postlease_lineage.into_parts();
    let PreliminaryResolutionRequestsPlannedWork {
        discovered,
        context,
        pe_material,
        plan,
    } = preliminary;
    let LaunchPathDiscoveredWork {
        admitted,
        candidates,
    } = discovered;
    Ok((
        admitted,
        QueryVerifiedWindowsRunnerLaunchLineage {
            context,
            candidates,
            pe_material,
            plan,
            grant_ready_plan,
        },
        PostLeaseSplitWindowsRunnerLoadSetPrerequisite {
            namespace,
            resolution,
        },
        package_content_leases,
    ))
}

fn validate_query_verified_lineage(
    prerequisite: &SealedWindowsRunnerLoadSetPrerequisite<'_>,
) -> Result<()> {
    prerequisite.postlease_lineage.validate_retained_plans()?;
    let preliminary = prerequisite.postlease_lineage.borrow_preliminary();
    let rebound = binding::bind_preliminary_request_plan(
        &preliminary.discovered,
        &preliminary.context,
        &preliminary.pe_material,
    )?;
    let plan = &preliminary.plan;
    let resolution = &prerequisite.resolution;
    let namespace = &prerequisite.namespace;
    let cross_binding = &resolution.pe_import_graph.pre_post_cross_binding;
    validate_pre_post_parsed_image_cross_bindings(
        &preliminary.pe_material,
        resolution,
        &prerequisite.package_content_leases,
    )?;
    validate_pre_post_import_edge_cross_bindings(&preliminary.plan, resolution)?;
    prerequisite
        .postlease_lineage
        .validate_final_system_image_projection(resolution)?;
    if plan.recompute_digest() != plan.preliminary_request_plan_digest
        || rebound.preliminary_request_plan_digest != plan.preliminary_request_plan_digest
        || plan.selected_context.context_intent_digest != preliminary.context.context_intent_digest
        || resolution.admission_source_digest != plan.admission_source_digest
        || resolution.admission_receipt_digest != plan.admission_receipt_digest
        || resolution.extraction_plan_digest != plan.extraction_plan_digest
        || resolution.extraction_evidence_digest != plan.extraction_evidence_digest
        || resolution.runner_relative_path != preliminary.context.runner_relative_path
        || resolution.working_directory_relative_path
            != plan.selected_context.working_directory_relative_path
        || resolution.working_directory_identity_digest
            != plan.selected_context.working_directory_identity_digest
        || resolution.launch_context_selector_digest != preliminary.context.context_intent_digest
        || resolution.selected_context_binding_digest
            != plan.selected_context.selection_binding_digest
        || resolution.preliminary_resolution_request_plan_digest
            != plan.preliminary_request_plan_digest
        || resolution.process_machine_context_digest
            != preliminary.context.machine_context.context_policy_digest
        || resolution
            .preloaded_module_authority
            .process_machine_context_digest
            != preliminary.context.machine_context.context_policy_digest
        || resolution.preloaded_module_authority.module_set_digest
            != plan.authenticated_preloaded_module_set_digest
        || namespace.resolution_profile_digest != resolution.resolution_profile_digest
        || namespace.preliminary_resolution_request_plan_digest
            != plan.preliminary_request_plan_digest
        || namespace.grant_ready_resolution_plan_digest
            != resolution.grant_ready_resolution_plan_digest
        || prerequisite.postlease_lineage.grant_ready_plan_digest()
            != resolution.grant_ready_resolution_plan_digest
        || cross_binding.prelease_material_set_digest != plan.prelease_pe_material_digest
        || cross_binding.postlease_parsed_image_set_digest
            != resolution.pe_import_graph.parsed_image_set_digest
        || cross_binding.postlease_import_edge_set_digest
            != resolution.pe_import_graph.import_edge_set_digest
        || cross_binding.postlease_reachable_node_set_digest
            != resolution.pe_import_graph.reachable_node_set_digest
        || cross_binding.package_content_lease_set_digest
            != resolution.package_content_lease_set_digest
        || super::super::super::digest::pe_pre_post_cross_binding_receipt_digest(cross_binding)?
            != cross_binding.receipt_digest
        || [
            &cross_binding.same_retained_file_handle_set_digest,
            &cross_binding.receipt_digest,
        ]
        .into_iter()
        .any(|digest| {
            !crate::node_agent_compute_plugin_host::manifest_validation::is_sha256(digest)
        })
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_QUERY_VERIFIED_LAUNCH_LINEAGE_CHANGED");
    }
    Ok(())
}

fn validate_pre_post_parsed_image_cross_bindings(
    prelease: &AuthenticatedWindowsPreLeasePeMaterial,
    resolution: &super::super::super::resolution::SealedWindowsLoaderResolutionAuthority,
    package_content_leases: &[WindowsLoaderPackageContentLeaseCustody],
) -> Result<()> {
    let receipt = &resolution.pe_import_graph.pre_post_cross_binding;
    if receipt.parsed_image_cross_bindings.len() != prelease.package_images().len()
        || super::super::super::digest::pe_parsed_image_cross_binding_set_digest(
            &receipt.parsed_image_cross_bindings,
        )? != receipt.parsed_image_cross_binding_set_digest
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_PE_PARSED_IMAGE_CROSS_BINDING_SET_CHANGED");
    }
    let mut same_handle_set =
        super::digest::PlanDigest::new(b"ELON_WINDOWS_SAME_RETAINED_PACKAGE_FILE_HANDLE_SET_V1");
    for (ordinal, cross) in receipt.parsed_image_cross_bindings.iter().enumerate() {
        let Some(prelease_image) = prelease
            .package_images()
            .get(cross.prelease_parsed_image_ordinal)
        else {
            bail!("COMPUTE_PLUGIN_WINDOWS_PE_PRELEASE_IMAGE_CROSS_BINDING_MISSING");
        };
        let Some(postlease_image) = resolution
            .pe_import_graph
            .parsed_images
            .get(cross.postlease_parsed_image_ordinal)
        else {
            bail!("COMPUTE_PLUGIN_WINDOWS_PE_POSTLEASE_IMAGE_CROSS_BINDING_MISSING");
        };
        let Some(lease) = package_content_leases
            .iter()
            .find(|lease| lease.package_file_ordinal == cross.package_file_ordinal)
        else {
            bail!("COMPUTE_PLUGIN_WINDOWS_PE_PACKAGE_LEASE_CROSS_BINDING_MISSING");
        };
        let lease_binding = lease.lease.binding();
        let expected_postlease_material =
            super::super::super::pe_graph_validation::package_parsed_image_material_digest(
                lease_binding.0,
                lease_binding.1,
                lease_binding.2,
                lease_binding.3,
            )?;
        let postlease_node_matches = matches!(
            &postlease_image.node,
            super::super::super::resolution::WindowsLoaderModuleNode::PackageFile {
                package_file_ordinal
            } if *package_file_ordinal == cross.package_file_ordinal
        );
        if cross.prelease_parsed_image_ordinal != ordinal
            || prelease_image.parsed_image_ordinal() != cross.prelease_parsed_image_ordinal
            || prelease_image.package_file_ordinal() != cross.package_file_ordinal
            || prelease_image.file_identity_digest() != cross.file_identity_digest
            || prelease_image.sealed_file_digest() != lease_binding.1
            || lease_binding.0 != cross.file_identity_digest
            || lease_binding.2 != cross.lease_generation_digest
            || postlease_image.parsed_image_ordinal != cross.postlease_parsed_image_ordinal
            || postlease_image.image_material_identity_digest
                != cross.postlease_image_material_identity_digest
            || postlease_image.image_material_identity_digest != expected_postlease_material
            || !postlease_node_matches
            || receipt.parsed_image_cross_bindings[..ordinal]
                .iter()
                .any(|prior| {
                    prior.package_file_ordinal == cross.package_file_ordinal
                        || prior.postlease_parsed_image_ordinal
                            == cross.postlease_parsed_image_ordinal
                })
            || [
                &cross.file_identity_digest,
                &cross.postlease_image_material_identity_digest,
                &cross.lease_generation_digest,
            ]
            .into_iter()
            .any(|digest| {
                !crate::node_agent_compute_plugin_host::manifest_validation::is_sha256(digest)
            })
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PE_PARSED_IMAGE_CROSS_BINDING_CHANGED");
        }
        same_handle_set.usize(cross.prelease_parsed_image_ordinal);
        same_handle_set.usize(cross.package_file_ordinal);
        same_handle_set.text(&cross.file_identity_digest);
        same_handle_set.usize(cross.postlease_parsed_image_ordinal);
        same_handle_set.text(&cross.postlease_image_material_identity_digest);
        same_handle_set.text(&cross.lease_generation_digest);
    }
    if same_handle_set.finish() != receipt.same_retained_file_handle_set_digest {
        bail!("COMPUTE_PLUGIN_WINDOWS_PE_SAME_HANDLE_SET_CHANGED");
    }
    Ok(())
}

fn validate_pre_post_import_edge_cross_bindings(
    preliminary: &PreliminaryWindowsRunnerResolutionRequestPlan,
    resolution: &super::super::super::resolution::SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    use super::super::super::resolution::WindowsLoaderImportBindingRef;

    let receipt = &resolution.pe_import_graph.pre_post_cross_binding;
    if receipt.import_edge_cross_bindings.len() != preliminary.module_resolution_requests.len()
        || super::super::super::digest::pe_import_edge_cross_binding_set_digest(
            &receipt.import_edge_cross_bindings,
        )? != receipt.import_edge_cross_binding_set_digest
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_PE_IMPORT_EDGE_CROSS_BINDING_SET_CHANGED");
    }
    let mut final_refs = std::collections::HashSet::new();
    for (ordinal, (cross, request)) in receipt
        .import_edge_cross_bindings
        .iter()
        .zip(&preliminary.module_resolution_requests)
        .enumerate()
    {
        let (
            request_ordinal,
            _,
            edge_locator,
            importer_graph_edge_ordinal,
            prelease_importer_parsed_image_ordinal,
            _,
            _,
            _,
            _,
            _,
            _,
        ) = request.request_binding();
        let Some(image_cross) = receipt.parsed_image_cross_bindings.iter().find(|binding| {
            binding.prelease_parsed_image_ordinal == prelease_importer_parsed_image_ordinal
        }) else {
            bail!("COMPUTE_PLUGIN_WINDOWS_PE_IMPORTER_CROSS_BINDING_MISSING");
        };
        let (ref_kind, ref_ordinal, final_matches) = match cross.postlease_import_binding {
            WindowsLoaderImportBindingRef::Package { binding_ordinal } => (
                "package",
                binding_ordinal,
                resolution
                    .package_module_bindings
                    .get(binding_ordinal)
                    .is_some_and(|binding| {
                        binding.module_request_ordinal == request_ordinal
                            && final_base_edge_locator_matches(
                                &binding.edge_locator,
                                request_ordinal,
                                edge_locator,
                            )
                            && binding.importer_graph_edge_ordinal == importer_graph_edge_ordinal
                            && binding.importer_parsed_image_ordinal
                                == cross.postlease_importer_parsed_image_ordinal
                    }),
            ),
            WindowsLoaderImportBindingRef::System { binding_ordinal } => (
                "system",
                binding_ordinal,
                resolution
                    .system_module_bindings
                    .get(binding_ordinal)
                    .is_some_and(|binding| {
                        binding.module_request_ordinal == request_ordinal
                            && final_base_edge_locator_matches(
                                &binding.edge_locator,
                                request_ordinal,
                                edge_locator,
                            )
                            && binding.importer_graph_edge_ordinal == importer_graph_edge_ordinal
                            && binding.importer_parsed_image_ordinal
                                == cross.postlease_importer_parsed_image_ordinal
                    }),
            ),
        };
        if cross.preliminary_request_ordinal != ordinal
            || request_ordinal != ordinal
            || cross.prelease_importer_parsed_image_ordinal
                != prelease_importer_parsed_image_ordinal
            || &cross.edge_locator != edge_locator
            || cross.postlease_importer_parsed_image_ordinal
                != image_cross.postlease_parsed_image_ordinal
            || !final_refs.insert((ref_kind, ref_ordinal))
            || !final_matches
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PE_IMPORT_EDGE_CROSS_BINDING_CHANGED");
        }
    }
    Ok(())
}

fn final_base_edge_locator_matches(
    final_locator: &super::super::super::resolution::WindowsLoaderModuleEdgeLocator,
    preliminary_request_ordinal: usize,
    preliminary_locator: &super::WindowsPreliminaryModuleEdgeLocator,
) -> bool {
    matches!(
        final_locator,
        super::super::super::resolution::WindowsLoaderModuleEdgeLocator::BasePrelease {
            preliminary_request_ordinal: final_request_ordinal,
            import_edge_cross_binding_ordinal,
            locator,
        } if *final_request_ordinal == preliminary_request_ordinal
            && *import_edge_cross_binding_ordinal == preliminary_request_ordinal
            && locator == preliminary_locator
    )
}

impl QueryVerifiedWindowsRunnerLaunchLineage {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_loader_image_binding(
        &self,
        admission_source_digest: &str,
        admission_receipt_digest: &str,
        profile: &ComputePluginWorkAdmissionLaunchProfile,
        image: &SealedComputePluginRunnerImage,
    ) -> Result<()> {
        self.context.validate_binding(
            admission_source_digest,
            admission_receipt_digest,
            profile,
        )?;
        let (candidate_set_digest, runner_file_ordinal, managed) = self.candidates.binding();
        let runner_file_identity_digest = managed.application().binding().1;
        self.pe_material.validate_binding(
            admission_source_digest,
            admission_receipt_digest,
            &self.plan.extraction_plan_digest,
            &self.plan.extraction_evidence_digest,
            candidate_set_digest,
            runner_file_ordinal,
            runner_file_identity_digest,
            &self.context.target_architecture,
            &self.context.machine_context.context_policy_digest,
        )?;
        let selected = &self.plan.selected_context;
        let working_directory_owner_matches = match selected.working_directory_location {
            WindowsLoaderWorkingDirectoryLocation::PackageRoot => {
                image.working_directory_location
                    == WindowsLoaderWorkingDirectoryLocation::PackageRoot
                    && selected.working_directory_relative_path == "."
                    && image
                        .package_root_directory
                        .matches_sealed_identity(&selected.working_directory_identity_digest)
            }
            WindowsLoaderWorkingDirectoryLocation::PlanDirectory { directory_ordinal } => {
                image.working_directory_location
                    == WindowsLoaderWorkingDirectoryLocation::PlanDirectory { directory_ordinal }
                    && image.namespace_directories.iter().any(|entry| {
                        entry.directory_ordinal == directory_ordinal
                            && entry.relative_path == selected.working_directory_relative_path
                            && entry.directory.matches_sealed_identity(
                                &selected.working_directory_identity_digest,
                            )
                    })
            }
        };
        let application_directory_owner_matches = match selected.application_directory_location {
            super::WindowsPreliminaryRetainedDirectoryLocation::PackageRoot => image
                .package_root_directory
                .matches_sealed_identity(&selected.application_directory_identity_digest),
            super::WindowsPreliminaryRetainedDirectoryLocation::PlanDirectory {
                directory_ordinal,
            } => image.namespace_directories.iter().any(|entry| {
                entry.directory_ordinal == directory_ordinal
                    && entry
                        .directory
                        .matches_sealed_identity(&selected.application_directory_identity_digest)
            }),
        };
        let resolution = &image.load_set_authority.resolution;
        self.grant_ready_plan
            .validate_final_base_projection(resolution)?;
        let (base_modules, base_names, base_system_images) =
            self.grant_ready_plan.base_projection_shape();
        resolution
            .pe_import_graph
            .recursive_resolution_closure
            .validate_against(
                self.pe_material.package_images().len(),
                base_modules,
                base_names,
                base_system_images,
                &self.plan.selected_context.context_intent_digest,
                &self.plan.preliminary_request_plan_digest,
                &self.plan.parser_policy_digest,
                &self.plan.authenticated_preloaded_module_set_digest,
                &self.plan.resolution_route_order,
                resolution,
            )?;
        if self.plan.recompute_digest() != self.plan.preliminary_request_plan_digest
            || self.plan.launch_path_candidate_set_digest != candidate_set_digest
            || self.plan.prelease_pe_material_digest != self.pe_material.material_set_digest()
            || self.plan.selected_context.context_intent_digest
                != self.context.context_intent_digest
            || runner_file_ordinal != image.runner_ordinal
            || runner_file_identity_digest != image.file_identity_digest
            || selected.application_identity_digest != image.file_identity_digest
            || selected.working_directory_identity_digest != image.working_directory_identity_digest
            || resolution.launch_context_selector_digest != self.context.context_intent_digest
            || resolution.selected_context_binding_digest != selected.selection_binding_digest
            || resolution.preliminary_resolution_request_plan_digest
                != self.plan.preliminary_request_plan_digest
            || resolution.grant_ready_resolution_plan_digest != self.grant_ready_plan.digest()
            || !working_directory_owner_matches
            || !application_directory_owner_matches
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RETAINED_LAUNCH_LINEAGE_IMAGE_CHANGED");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_process_projection(
        &self,
        profile: &ComputePluginWorkAdmissionLaunchProfile,
        expected: &WindowsRunnerLaunchContextPreCreateProjection<'_>,
    ) -> Result<()> {
        self.context.validate_process_projection(
            &self.context.context_intent_digest,
            &self.plan.selected_context.working_directory_identity_digest,
            profile,
            expected,
        )
    }
}
