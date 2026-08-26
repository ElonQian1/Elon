//! Pure borrowed binding from retained discovery owners to a preliminary request plan.

use anyhow::{bail, Result};

use crate::{
    node_agent_compute_plugin_host::runtime_loader_load_set::model::WindowsLoaderWorkingDirectoryLocation,
    node_agent_managed_fs::{
        ManagedLoaderLaunchPathDiscoveryReceipt, ManagedLoaderLaunchPathDiscoverySet,
    },
};

use super::{
    digest::{working_directory_location_name, PlanDigest},
    AuthenticatedWindowsRunnerLaunchContextIntent,
    AuthenticatedWindowsRunnerWorkingDirectorySelector,
    PreliminaryWindowsRunnerResolutionRequestPlan, WindowsPreliminaryContentLeaseRequestRef,
    WindowsPreliminaryImportEdgeKind, WindowsPreliminaryLaunchPathComponentRequest,
    WindowsPreliminaryModuleEdgeLocator, WindowsPreliminaryModuleResolutionRequest,
    WindowsPreliminaryRetainedDirectoryLocation, WindowsPreliminarySearchDirectoryBinding,
    WindowsPreliminarySearchDirectoryRole, WindowsPreliminarySearchDirectoryTarget,
    WindowsRunnerSelectedLaunchContextBinding,
};
use crate::node_agent_compute_plugin_host::runtime_loader_load_set::launch_path_discovery::{
    prelease_pe_material::{AuthenticatedWindowsPreLeasePeMaterial, WindowsPreLeaseImportKind},
    LaunchPathDiscoveredWork,
};

pub(super) fn bind_preliminary_request_plan(
    discovered: &LaunchPathDiscoveredWork<'_>,
    context: &AuthenticatedWindowsRunnerLaunchContextIntent,
    pe_material: &AuthenticatedWindowsPreLeasePeMaterial,
) -> Result<PreliminaryWindowsRunnerResolutionRequestPlan> {
    let receipts = discovered.admitted.receipts();
    receipts.validate()?;
    let profile = receipts.source().source().launch_profile();
    profile.validate()?;
    let archive = discovered
        .admitted
        .installed()
        .revalidated()
        .staged()
        .archive();
    let view = archive.launch_path_discovery_view();
    let envelope = view.plan().envelope();
    let evidence = view.evidence();
    let (candidate_set_digest, runner_file_ordinal, managed) = discovered.candidates.binding();
    let runner_identity_digest = managed.application().binding().1;

    context.validate_binding(
        receipts.source().source_digest(),
        receipts.receipt().receipt_digest(),
        profile,
    )?;
    pe_material.validate_binding(
        receipts.source().source_digest(),
        receipts.receipt().receipt_digest(),
        &envelope.plan_digest,
        &evidence.evidence_digest,
        candidate_set_digest,
        runner_file_ordinal,
        runner_identity_digest,
        &context.target_architecture,
        &context.machine_context.context_policy_digest,
    )?;
    for image in pe_material.package_images() {
        let ordinal = image.package_file_ordinal();
        let planned = envelope
            .plan
            .files
            .get(ordinal)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_PE_PLAN_FILE_MISSING"))?;
        let observed =
            evidence.evidence.files.get(ordinal).ok_or_else(|| {
                anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_PE_EVIDENCE_FILE_MISSING")
            })?;
        let retained = view
            .files()
            .get(ordinal)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_PE_RETAINED_FILE_MISSING"))?;
        if image.relative_path() != planned.relative_path
            || image.relative_path() != observed.relative_path
            || image.sealed_file_digest() != planned.expected_digest
            || image.sealed_file_digest() != observed.digest
            || image.size_bytes() != u64::try_from(planned.expected_size_bytes)?
            || image.size_bytes() != u64::try_from(observed.size_bytes)?
            || image.size_bytes() != retained.len_bytes()
            || image.file_identity_digest() != observed.file_identity_digest
            || image.file_identity_digest() != retained.identity_digest()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRELEASE_PE_PACKAGE_FILE_CHANGED");
        }
    }

    let selected_context = select_context(managed, &envelope.plan.directories, context)?;
    let search_directories = bind_search_directories(managed, context, &selected_context)?;
    let module_resolution_requests =
        bind_module_resolution_requests(pe_material, &search_directories)?;
    let launch_path_component_requests = bind_launch_components(managed, &selected_context)?;
    let content_lease_requests = bind_content_lease_requests(envelope.plan.files.len());
    let mut plan = PreliminaryWindowsRunnerResolutionRequestPlan {
        admission_source_digest: receipts.source().source_digest().to_owned(),
        admission_receipt_digest: receipts.receipt().receipt_digest().to_owned(),
        extraction_plan_digest: envelope.plan_digest.clone(),
        extraction_evidence_digest: evidence.evidence_digest.clone(),
        launch_path_candidate_set_digest: candidate_set_digest.to_owned(),
        selected_context,
        prelease_pe_material_digest: pe_material.material_set_digest().to_owned(),
        parser_policy_digest: pe_material.parser_policy_digest().to_owned(),
        authenticated_preloaded_module_set_digest: pe_material
            .preloaded_module_set_digest()
            .to_owned(),
        resolution_route_order: context.dll_search_policy.resolution_route_order.clone(),
        search_directories,
        module_resolution_requests,
        launch_path_component_requests,
        content_lease_requests,
        preliminary_request_plan_digest: String::new(),
    };
    plan.preliminary_request_plan_digest = plan.recompute_digest();
    Ok(plan)
}

fn select_context(
    managed: &ManagedLoaderLaunchPathDiscoverySet,
    plan_directories: &[String],
    context: &AuthenticatedWindowsRunnerLaunchContextIntent,
) -> Result<WindowsRunnerSelectedLaunchContextBinding> {
    let application_directory =
        select_application_directory(managed, plan_directories, &context.runner_relative_path)?;
    let selected = match &context.working_directory_selector {
        AuthenticatedWindowsRunnerWorkingDirectorySelector::PackageRoot => (
            WindowsLoaderWorkingDirectoryLocation::PackageRoot,
            ".".to_owned(),
            managed.package_root(),
        ),
        AuthenticatedWindowsRunnerWorkingDirectorySelector::PlanDirectory {
            directory_ordinal,
            relative_path,
        } => {
            let entry = managed
                .plan_directories()
                .get(*directory_ordinal)
                .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_CWD_ORDINAL_MISSING"))?;
            let (candidate_ordinal, receipt) = entry.binding();
            if candidate_ordinal != *directory_ordinal
                || plan_directories.get(*directory_ordinal) != Some(relative_path)
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_CWD_SELECTOR_CHANGED");
            }
            (
                WindowsLoaderWorkingDirectoryLocation::PlanDirectory {
                    directory_ordinal: *directory_ordinal,
                },
                relative_path.clone(),
                receipt,
            )
        }
    };
    bind_selected_context(
        managed.application(),
        application_directory,
        selected,
        &context.context_intent_digest,
    )
}

fn select_application_directory<'managed>(
    managed: &'managed ManagedLoaderLaunchPathDiscoverySet,
    plan_directories: &[String],
    runner_relative_path: &str,
) -> Result<(
    WindowsPreliminaryRetainedDirectoryLocation,
    &'managed ManagedLoaderLaunchPathDiscoveryReceipt,
)> {
    let parent_relative_path = runner_relative_path
        .rsplit_once('/')
        .map_or(".", |(parent, _)| parent);
    if parent_relative_path == "." {
        return Ok((
            WindowsPreliminaryRetainedDirectoryLocation::PackageRoot,
            managed.package_root(),
        ));
    }
    let mut matches = plan_directories
        .iter()
        .enumerate()
        .filter(|(_, relative_path)| relative_path.as_str() == parent_relative_path);
    let (directory_ordinal, _) = matches
        .next()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_APPLICATION_DIRECTORY_MISSING"))?;
    if matches.next().is_some() {
        bail!("COMPUTE_PLUGIN_WINDOWS_APPLICATION_DIRECTORY_DUPLICATED");
    }
    let (candidate_ordinal, receipt) = managed
        .plan_directories()
        .get(directory_ordinal)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_APPLICATION_DIRECTORY_MISSING"))?
        .binding();
    if candidate_ordinal != directory_ordinal {
        bail!("COMPUTE_PLUGIN_WINDOWS_APPLICATION_DIRECTORY_ORDINAL_CHANGED");
    }
    Ok((
        WindowsPreliminaryRetainedDirectoryLocation::PlanDirectory { directory_ordinal },
        receipt,
    ))
}

fn bind_selected_context(
    application: &ManagedLoaderLaunchPathDiscoveryReceipt,
    application_directory: (
        WindowsPreliminaryRetainedDirectoryLocation,
        &ManagedLoaderLaunchPathDiscoveryReceipt,
    ),
    selected: (
        WindowsLoaderWorkingDirectoryLocation,
        String,
        &ManagedLoaderLaunchPathDiscoveryReceipt,
    ),
    context_intent_digest: &str,
) -> Result<WindowsRunnerSelectedLaunchContextBinding> {
    let application_components = application.components();
    let application_file = application_components
        .last()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_APPLICATION_COMPONENT_MISSING"))?;
    let application_directory_components = application_directory.1.components();
    let application_directory_binding = application_directory.1.binding();
    let application_directory_identity_digest = application_directory_binding.1.to_owned();
    let application_prefix = &application_components[..application_components.len() - 1];
    if application_file.binding().1 != application_directory_binding.1
        || application_prefix.len() != application_directory_components.len()
        || application_prefix
            .iter()
            .zip(application_directory_components)
            .any(|(from_file, from_directory)| {
                let from_file = from_file.binding();
                let from_directory = from_directory.binding();
                from_file.0 != from_directory.0
                    || from_file.1 != from_directory.1
                    || from_file.2 != from_directory.2
                    || from_file.3 != from_directory.3
            })
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_APPLICATION_DIRECTORY_BINDING_CHANGED");
    }
    let application_directory_component_set_digest = application_directory_binding.3.to_owned();
    let (_, application_identity, _, application_component_set, _, application_observation) =
        application.binding();
    let (_, cwd_identity, _, cwd_component_set, _, cwd_observation) = selected.2.binding();
    let mut digest = PlanDigest::new(b"ELON_WINDOWS_RUNNER_SELECTED_LAUNCH_CONTEXT_V1");
    for value in [
        context_intent_digest,
        application_identity,
        application_component_set,
        application_observation,
        &application_directory_identity_digest,
        &application_directory_component_set_digest,
        application_directory_binding.5,
        cwd_identity,
        cwd_component_set,
        cwd_observation,
        &selected.1,
    ] {
        digest.text(value);
    }
    digest.text(working_directory_location_name(&selected.0));
    Ok(WindowsRunnerSelectedLaunchContextBinding {
        working_directory_location: selected.0,
        working_directory_relative_path: selected.1,
        working_directory_identity_digest: cwd_identity.to_owned(),
        working_directory_component_set_digest: cwd_component_set.to_owned(),
        working_directory_observation_receipt_digest: cwd_observation.to_owned(),
        application_identity_digest: application_identity.to_owned(),
        application_component_set_digest: application_component_set.to_owned(),
        application_observation_receipt_digest: application_observation.to_owned(),
        application_directory_identity_digest,
        application_directory_location: application_directory.0,
        application_directory_component_set_digest,
        application_directory_observation_receipt_digest: application_directory_binding
            .5
            .to_owned(),
        context_intent_digest: context_intent_digest.to_owned(),
        selection_binding_digest: digest.finish(),
    })
}

fn bind_search_directories(
    managed: &ManagedLoaderLaunchPathDiscoverySet,
    context: &AuthenticatedWindowsRunnerLaunchContextIntent,
    selected: &WindowsRunnerSelectedLaunchContextBinding,
) -> Result<Vec<WindowsPreliminarySearchDirectoryBinding>> {
    context
        .dll_search_policy
        .search_order
        .iter()
        .enumerate()
        .map(|(ordinal, role)| {
            let target = match role {
                WindowsPreliminarySearchDirectoryRole::ApplicationDirectory => {
                    WindowsPreliminarySearchDirectoryTarget::RetainedCandidate {
                        location: selected.application_directory_location,
                        identity_digest: selected.application_directory_identity_digest.clone(),
                        observation_receipt_digest: selected
                            .application_directory_observation_receipt_digest
                            .clone(),
                    }
                }
                WindowsPreliminarySearchDirectoryRole::CurrentDirectory => {
                    WindowsPreliminarySearchDirectoryTarget::RetainedCandidate {
                        location: match selected.working_directory_location {
                            WindowsLoaderWorkingDirectoryLocation::PackageRoot => {
                                WindowsPreliminaryRetainedDirectoryLocation::PackageRoot
                            }
                            WindowsLoaderWorkingDirectoryLocation::PlanDirectory {
                                directory_ordinal,
                            } => WindowsPreliminaryRetainedDirectoryLocation::PlanDirectory {
                                directory_ordinal,
                            },
                        },
                        identity_digest: selected.working_directory_identity_digest.clone(),
                        observation_receipt_digest: selected
                            .working_directory_observation_receipt_digest
                            .clone(),
                    }
                }
                WindowsPreliminarySearchDirectoryRole::PackageRoot => {
                    let binding = managed.package_root().binding();
                    WindowsPreliminarySearchDirectoryTarget::RetainedCandidate {
                        location: WindowsPreliminaryRetainedDirectoryLocation::PackageRoot,
                        identity_digest: binding.1.to_owned(),
                        observation_receipt_digest: binding.5.to_owned(),
                    }
                }
                WindowsPreliminarySearchDirectoryRole::PlanDirectory { directory_ordinal } => {
                    let entry = managed
                        .plan_directories()
                        .get(*directory_ordinal)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "COMPUTE_PLUGIN_WINDOWS_SEARCH_DIRECTORY_ORDINAL_MISSING"
                            )
                        })?;
                    let (candidate_ordinal, receipt) = entry.binding();
                    if candidate_ordinal != *directory_ordinal {
                        bail!("COMPUTE_PLUGIN_WINDOWS_SEARCH_DIRECTORY_ORDINAL_CHANGED");
                    }
                    let binding = receipt.binding();
                    WindowsPreliminarySearchDirectoryTarget::RetainedCandidate {
                        location: WindowsPreliminaryRetainedDirectoryLocation::PlanDirectory {
                            directory_ordinal: *directory_ordinal,
                        },
                        identity_digest: binding.1.to_owned(),
                        observation_receipt_digest: binding.5.to_owned(),
                    }
                }
                WindowsPreliminarySearchDirectoryRole::SystemDirectory => {
                    WindowsPreliminarySearchDirectoryTarget::ExternalTypedOwnerRequired {
                        owner_kind: "system_directory",
                    }
                }
                WindowsPreliminarySearchDirectoryRole::WindowsDirectory => {
                    WindowsPreliminarySearchDirectoryTarget::ExternalTypedOwnerRequired {
                        owner_kind: "windows_directory",
                    }
                }
                WindowsPreliminarySearchDirectoryRole::SideBySideAssemblyDirectory => {
                    WindowsPreliminarySearchDirectoryTarget::ExternalTypedOwnerRequired {
                        owner_kind: "side_by_side_assembly_directory",
                    }
                }
            };
            let role_name = role.unique_key();
            let mut digest = PlanDigest::new(b"ELON_WINDOWS_PRELIMINARY_SEARCH_DIRECTORY_V1");
            digest.usize(ordinal);
            digest.text(&role_name);
            match &target {
                WindowsPreliminarySearchDirectoryTarget::RetainedCandidate {
                    location,
                    identity_digest,
                    observation_receipt_digest,
                } => {
                    digest.text("retained_candidate");
                    match location {
                        WindowsPreliminaryRetainedDirectoryLocation::PackageRoot => {
                            digest.text("package_root");
                        }
                        WindowsPreliminaryRetainedDirectoryLocation::PlanDirectory {
                            directory_ordinal,
                        } => {
                            digest.text("plan_directory");
                            digest.usize(*directory_ordinal);
                        }
                    }
                    digest.text(identity_digest);
                    digest.text(observation_receipt_digest);
                }
                WindowsPreliminarySearchDirectoryTarget::ExternalTypedOwnerRequired {
                    owner_kind,
                } => {
                    digest.text("external_typed_owner_required");
                    digest.text(owner_kind);
                }
            }
            Ok(WindowsPreliminarySearchDirectoryBinding {
                search_step_ordinal: ordinal,
                role: role_name,
                target,
                binding_digest: digest.finish(),
            })
        })
        .collect()
}

fn bind_module_resolution_requests(
    pe_material: &AuthenticatedWindowsPreLeasePeMaterial,
    directories: &[WindowsPreliminarySearchDirectoryBinding],
) -> Result<Vec<WindowsPreliminaryModuleResolutionRequest>> {
    let ordered_search_step_ordinals = directories
        .iter()
        .map(|directory| directory.search_step_ordinal)
        .collect::<Vec<_>>();
    let mut requests: Vec<WindowsPreliminaryModuleResolutionRequest> = Vec::new();

    // Every PE import-table edge remains an exact Import request even when the resolved export
    // later forwards elsewhere. Forwarder hops are appended in their own evidence domain below.
    for edge in pe_material.import_edges() {
        let request_ordinal = requests.len();
        let importer_image_ordinal = edge.importer_image_ordinal();
        let importer_graph_edge_ordinal = requests
            .iter()
            .filter(|request| request.importer_image_ordinal == importer_image_ordinal)
            .count();
        let (descriptor_ordinal, thunk_ordinal) = edge.descriptor_and_thunk_ordinals();
        let (imported_symbol_name, imported_symbol_ordinal) = edge.imported_symbol_binding();
        requests.push(WindowsPreliminaryModuleResolutionRequest {
            request_ordinal,
            global_import_edge_ordinal: request_ordinal,
            edge_locator: WindowsPreliminaryModuleEdgeLocator::Import {
                source_import_edge_ordinal: edge.edge_ordinal(),
                descriptor_ordinal,
                thunk_ordinal,
                edge_evidence_digest: edge.edge_evidence_digest().to_owned(),
            },
            importer_graph_edge_ordinal,
            importer_image_ordinal,
            import_kind: match edge.import_kind() {
                WindowsPreLeaseImportKind::Normal => WindowsPreliminaryImportEdgeKind::Normal,
                WindowsPreLeaseImportKind::Delay => WindowsPreliminaryImportEdgeKind::Delay,
            },
            normalized_name: edge.normalized_module_name().to_owned(),
            imported_symbol_name: imported_symbol_name.map(str::to_owned),
            imported_symbol_ordinal,
            ordered_search_step_ordinals: ordered_search_step_ordinals.clone(),
            grant_ready_resolution_status:
                "exact_terminal_and_step_dispositions_required_before_grant",
        });
    }

    for hop in pe_material.forwarder_hops() {
        let (source_import_edge_ordinal, forwarder_hop_ordinal) =
            hop.source_edge_and_hop_ordinals();
        if pe_material
            .import_edges()
            .get(source_import_edge_ordinal)
            .is_none()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_FORWARDER_SOURCE_EDGE_MISSING");
        }
        let request_ordinal = requests.len();
        let importer_image_ordinal = hop.source_image_ordinal();
        let importer_graph_edge_ordinal = requests
            .iter()
            .filter(|request| request.importer_image_ordinal == importer_image_ordinal)
            .count();
        let (source_export_name, source_export_ordinal) = hop.source_symbol_binding();
        let (target_symbol_name, target_symbol_ordinal) = hop.target_symbol_binding();
        requests.push(WindowsPreliminaryModuleResolutionRequest {
            request_ordinal,
            global_import_edge_ordinal: request_ordinal,
            edge_locator: WindowsPreliminaryModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                forwarder_hop_ordinal,
                source_export_name: source_export_name.map(str::to_owned),
                source_export_ordinal,
                hop_evidence_digest: hop.hop_evidence_digest().to_owned(),
            },
            importer_graph_edge_ordinal,
            importer_image_ordinal,
            import_kind: WindowsPreliminaryImportEdgeKind::Forwarder,
            normalized_name: hop.target_module_name().to_owned(),
            imported_symbol_name: target_symbol_name.map(str::to_owned),
            imported_symbol_ordinal: target_symbol_ordinal,
            ordered_search_step_ordinals: ordered_search_step_ordinals.clone(),
            grant_ready_resolution_status:
                "exact_forwarder_hop_terminal_and_step_dispositions_required_before_grant",
        });
    }

    Ok(requests)
}

fn bind_launch_components(
    managed: &ManagedLoaderLaunchPathDiscoverySet,
    selected: &WindowsRunnerSelectedLaunchContextBinding,
) -> Result<Vec<WindowsPreliminaryLaunchPathComponentRequest>> {
    let cwd = match selected.working_directory_location {
        WindowsLoaderWorkingDirectoryLocation::PackageRoot => managed.package_root(),
        WindowsLoaderWorkingDirectoryLocation::PlanDirectory { directory_ordinal } => managed
            .plan_directories()
            .get(directory_ordinal)
            .map(|entry| entry.binding().1)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_CWD_COMPONENTS_MISSING"))?,
    };
    let mut requests = Vec::new();
    append_component_requests(&mut requests, "application", managed.application());
    append_component_requests(&mut requests, "working_directory", cwd);
    Ok(requests)
}

fn append_component_requests(
    requests: &mut Vec<WindowsPreliminaryLaunchPathComponentRequest>,
    path_kind: &'static str,
    receipt: &ManagedLoaderLaunchPathDiscoveryReceipt,
) {
    for component in receipt.components() {
        let binding = component.binding();
        requests.push(WindowsPreliminaryLaunchPathComponentRequest {
            request_ordinal: requests.len(),
            path_kind,
            component_ordinal: binding.0,
            parent_identity_digest: binding.1.to_owned(),
            normalized_component: binding.2.to_owned(),
            expected_object_identity_digest: binding.3.to_owned(),
        });
    }
}

fn bind_content_lease_requests(
    package_file_count: usize,
) -> Vec<WindowsPreliminaryContentLeaseRequestRef> {
    (0..package_file_count)
        .map(
            |package_file_ordinal| WindowsPreliminaryContentLeaseRequestRef::PackageFile {
                package_file_ordinal,
            },
        )
        .collect()
}
