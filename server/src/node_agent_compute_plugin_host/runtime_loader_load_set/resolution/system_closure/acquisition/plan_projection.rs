//! Independent final-graph cross-binding for retained recursive pre-dispatch typed plans.

use anyhow::{anyhow, bail, Result};

use super::super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderApiSetHostResolution,
    WindowsLoaderImportBindingRef, WindowsLoaderImportEdgeKind, WindowsLoaderModuleEdgeLocator,
    WindowsLoaderModuleNode, WindowsLoaderSearchedNameDisposition,
    WindowsLoaderSystemModuleBinding, WindowsLoaderSystemResolutionOrigin,
};
use super::super::{
    SealedWindowsRecursiveResolutionClosure, WindowsPeParsedImageSource,
    WindowsRecursiveResolutionWavePlan,
};
use super::{
    plan::*, plan_digest, WindowsRecursiveAcquisitionPlanEvidence,
    WindowsRecursiveWaveAcquisitionReceipt,
};

pub(super) fn validate_recursive_plan_evidence_against(
    receipt: &WindowsRecursiveWaveAcquisitionReceipt,
    wave: &WindowsRecursiveResolutionWavePlan,
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    let WindowsRecursiveAcquisitionPlanEvidence::RecursiveWave { plan } =
        &receipt.pre_dispatch_plan_evidence
    else {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PLAN_EVIDENCE_KIND_CHANGED");
    };
    validate_evidence_digests(plan)?;
    let source_frontier = plan
        .source_frontier
        .iter()
        .map(|source| source.parse_receipt_ordinal)
        .collect::<Vec<_>>();
    let next_frontier = plan
        .route_owners
        .iter()
        .filter_map(|owner| match &owner.parse_disposition {
            WindowsRecursiveTargetParseDisposition::NextFrontier {
                parse_receipt_ordinal,
                ..
            } => Some(*parse_receipt_ordinal),
            WindowsRecursiveTargetParseDisposition::AlreadyParsed { .. } => None,
        })
        .collect::<Vec<_>>();
    if plan.producer_wave_ordinal != wave.wave_ordinal
        || plan.producer_wave_ordinal != receipt.producer_wave_ordinal
        || plan.previous_acquisition_receipt_digest
            != receipt
                .previous_acquisition_receipt_digest
                .as_deref()
                .unwrap_or_default()
        || plan.input_custody_digest != receipt.input_custody_digest
        || plan.authenticated_recursive_policy_digest
            != receipt.authenticated_recursive_policy_digest
        || plan.parser_policy_digest != receipt.parser_policy_digest
        || source_frontier != wave.source_parse_receipt_ordinals
        || source_frontier != receipt.source_frontier_parse_receipt_ordinals
        || plan.first_module_request_ordinal != wave.first_module_request_ordinal
        || plan.module_requests.len() != wave.module_request_count
        || plan.module_resolutions.len() != plan.module_requests.len()
        || plan.first_searched_name_ordinal != wave.first_searched_name_ordinal
        || plan.searched_name_dispositions.len() != wave.searched_name_count
        || plan.first_system_image_request_ordinal != wave.first_system_image_request_ordinal
        || plan.filesystem_image_requests.len() != wave.system_image_request_count
        || next_frontier != wave.next_frontier_parse_receipt_ordinals
        || next_frontier != receipt.next_frontier_parse_receipt_ordinals
        || plan.request_plan_digest != receipt.source_request_plan_digest
        || plan.resolved_plan_digest != receipt.resolved_plan_digest
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PLAN_EVIDENCE_RANGE_CHANGED");
    }
    validate_source_frontier(plan, closure)?;
    validate_module_projection(plan, resolution)?;
    validate_search_projection(plan, resolution)?;
    validate_filesystem_projection(plan, resolution)?;
    validate_owner_projection(plan, closure, resolution)
}

fn validate_evidence_digests(plan: &WindowsRecursiveWaveDispatchPlanEvidence) -> Result<()> {
    if plan.request_plan_digest != plan_digest::request_plan_evidence_digest(plan)?
        || plan.terminal_resolution_set_digest
            != plan_digest::terminal_resolution_set_digest(
                plan.producer_wave_ordinal,
                &plan.module_resolutions,
            )?
        || plan.searched_name_disposition_set_digest
            != plan_digest::searched_name_disposition_set_digest(
                plan.producer_wave_ordinal,
                &plan.searched_name_dispositions,
            )?
        || plan.filesystem_request_set_digest
            != plan_digest::filesystem_request_set_digest(
                plan.producer_wave_ordinal,
                &plan.filesystem_image_requests,
            )?
        || plan.route_owner_set_digest
            != plan_digest::route_owner_set_digest(plan.producer_wave_ordinal, &plan.route_owners)?
        || plan.resolved_plan_digest != plan_digest::resolved_plan_evidence_digest(plan)?
        || plan.validated_plan_evidence_digest
            != plan_digest::validated_dispatch_plan_evidence_digest(plan)?
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_VALIDATED_PLAN_EVIDENCE_CHANGED");
    }
    Ok(())
}

fn validate_source_frontier(
    plan: &WindowsRecursiveWaveDispatchPlanEvidence,
    closure: &SealedWindowsRecursiveResolutionClosure,
) -> Result<()> {
    for source in &plan.source_frontier {
        let receipt = closure
            .parse_receipts
            .get(source.parse_receipt_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_SOURCE_MISSING"))?;
        if !source.matches_receipt(receipt) {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_SOURCE_CHANGED");
        }
    }
    Ok(())
}

fn validate_module_projection(
    plan: &WindowsRecursiveWaveDispatchPlanEvidence,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    for (request, module) in plan.module_requests.iter().zip(&plan.module_resolutions) {
        let package = resolution
            .package_module_bindings
            .iter()
            .find(|binding| binding.module_request_ordinal == request.module_request_ordinal);
        let system = resolution
            .system_module_bindings
            .iter()
            .find(|binding| binding.module_request_ordinal == request.module_request_ordinal);
        if package.is_some() == system.is_some() {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_MODULE_CARDINALITY_CHANGED");
        }
        let common_matches = package.is_some_and(|binding| {
            final_common_matches(
                request,
                plan.producer_wave_ordinal,
                binding.global_import_edge_ordinal,
                &binding.edge_locator,
                binding.importer_parsed_image_ordinal,
                &binding.importer,
                binding.importer_graph_edge_ordinal,
                binding.edge_kind,
                &binding.normalized_import_name,
                binding.imported_symbol_name.as_deref(),
                binding.imported_symbol_ordinal,
            ) && matches!(
                &module.terminal,
                WindowsRecursiveModuleTerminalRef::Direct {
                    owner: WindowsRecursiveRouteOwnerRef::PackageContentLease {
                        package_file_ordinal
                    }
                } if *package_file_ordinal == binding.resolved_package_file_ordinal
            )
        }) || system.is_some_and(|binding| {
            final_common_matches(
                request,
                plan.producer_wave_ordinal,
                binding.global_import_edge_ordinal,
                &binding.edge_locator,
                binding.importer_parsed_image_ordinal,
                &binding.importer,
                binding.importer_graph_edge_ordinal,
                binding.edge_kind,
                &binding.normalized_import_name,
                binding.imported_symbol_name.as_deref(),
                binding.imported_symbol_ordinal,
            ) && system_terminal_matches(&module.terminal, binding, resolution)
        });
        let sequence = resolution
            .pe_import_graph
            .search_sequences
            .get(request.module_request_ordinal);
        if module.module_request_ordinal != request.module_request_ordinal
            || !common_matches
            || !sequence.is_some_and(|sequence| {
                sequence.sequence_ordinal == request.module_request_ordinal
                    && sequence.searched_name_ordinals == module.searched_name_ordinals
            })
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_MODULE_CHANGED");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn final_common_matches(
    request: &WindowsRecursiveParsedEdgeRequest,
    producer_wave_ordinal: usize,
    global_import_edge_ordinal: usize,
    locator: &WindowsLoaderModuleEdgeLocator,
    importer_parsed_image_ordinal: usize,
    importer: &WindowsLoaderModuleNode,
    importer_graph_edge_ordinal: usize,
    edge_kind: WindowsLoaderImportEdgeKind,
    normalized_import_name: &str,
    imported_symbol_name: Option<&str>,
    imported_symbol_ordinal: Option<u16>,
) -> bool {
    matches!(
        locator,
        WindowsLoaderModuleEdgeLocator::SystemPostLease {
            wave_ordinal,
            source_parsed_image_ordinal,
            parse_receipt_ordinal,
            locator,
        } if *wave_ordinal == producer_wave_ordinal
            && *source_parsed_image_ordinal == request.importer_parsed_image_ordinal
            && *parse_receipt_ordinal == request.source_parse_receipt_ordinal
            && locator == &request.edge_locator
    ) && global_import_edge_ordinal == request.global_import_edge_ordinal
        && importer_parsed_image_ordinal == request.importer_parsed_image_ordinal
        && importer == &request.importer
        && importer_graph_edge_ordinal == request.importer_graph_edge_ordinal
        && final_import_kind_matches(&request.import_kind, edge_kind)
        && normalized_import_name == request.normalized_requested_name
        && imported_symbol_name == request.imported_symbol_name.as_deref()
        && imported_symbol_ordinal == request.imported_symbol_ordinal
}

fn validate_search_projection(
    plan: &WindowsRecursiveWaveDispatchPlanEvidence,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    for searched in &plan.searched_name_dispositions {
        let final_name = resolution
            .searched_names
            .get(searched.searched_name_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_SEARCH_MISSING"))?;
        let module = plan
            .module_resolutions
            .iter()
            .find(|module| module.module_request_ordinal == searched.module_request_ordinal)
            .ok_or_else(|| {
                anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_SEARCH_MODULE_MISSING")
            })?;
        let disposition_matches = match (&searched.disposition, &final_name.disposition) {
            (
                WindowsRecursiveSearchedNameDisposition::MustRemainAbsent,
                WindowsLoaderSearchedNameDisposition::MustRemainAbsent,
            ) => true,
            (
                WindowsRecursiveSearchedNameDisposition::Terminal { terminal },
                WindowsLoaderSearchedNameDisposition::ExpectedPackage { .. }
                | WindowsLoaderSearchedNameDisposition::ExpectedSystem { .. },
            ) => terminal == &module.terminal,
            _ => false,
        };
        if import_binding_module_request_ordinal(&final_name.import_binding, resolution)
            != Some(searched.module_request_ordinal)
            || final_name.search_step_ordinal != searched.step_position
            || final_name.search_directory_ordinal != searched.search_directory_ordinal
            || final_name.normalized_name != searched.normalized_name
            || final_name.search_directory_authority_binding_digest
                != searched.search_directory_authority_binding_digest
            || final_name.grant_request_digest != searched.grant_request_digest
            || final_name.disposition_binding_digest != searched.disposition_binding_digest
            || !disposition_matches
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_SEARCH_CHANGED");
        }
    }
    Ok(())
}

fn validate_filesystem_projection(
    plan: &WindowsRecursiveWaveDispatchPlanEvidence,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    for request in &plan.filesystem_image_requests {
        let custody = resolution
            .resolved_filesystem_system_images
            .get(request.resolution_request_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_FILESYSTEM_MISSING"))?;
        let primary_use = request
            .uses
            .get(request.primary_use_ordinal)
            .ok_or_else(|| {
                anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_FILESYSTEM_PRIMARY_USE_MISSING")
            })?;
        let directory = resolution
            .search_directories
            .get(primary_use.search_directory_ordinal)
            .ok_or_else(|| {
                anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_FILESYSTEM_DIRECTORY_MISSING")
            })?;
        let component_image = resolution
            .system_module_images
            .component_images
            .iter()
            .find(|image| {
                image.component_identity_digest == request.resolved_component_identity_digest
                    && image.image_file_identity_digest == request.expected_file_identity_digest
                    && image.code_integrity_evidence_digest
                        == request.code_integrity_evidence_digest
                    && image.servicing_generation_digest
                        == request.concrete_servicing_generation_digest
            })
            .ok_or_else(|| {
                anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_FILESYSTEM_COMPONENT_MISSING")
            })?;
        let (outcome_request, candidate, _, lease_request, _, _, _) = custody.outcome.binding();
        let (_, normalized_name, file_identity, _, _, _) = custody.outcome.image().binding();
        let (_, _, servicing_generation, _, _) = custody.outcome.image().content_lease_binding();
        if custody.resolution_request_ordinal != request.resolution_request_ordinal
            || outcome_request != request.resolution_request_ordinal
            || candidate != request.candidate_binding_digest
            || lease_request != request.lease_request_digest
            || normalized_name != request.normalized_name
            || file_identity != request.expected_file_identity_digest
            || servicing_generation != request.concrete_servicing_generation_digest
            || directory.search_directory_ordinal != primary_use.search_directory_ordinal
            || directory.policy_source_digest
                != primary_use.search_directory_authority_binding_digest
            || !custody.outcome.matches_resolution_request(
                request.resolution_request_ordinal,
                &request.candidate_binding_digest,
                &request.lease_request_digest,
                &directory.directory_identity_digest,
                &request.normalized_name,
                &request.expected_file_identity_digest,
                &component_image.immutable_section_identity_digest,
                &request.concrete_servicing_generation_digest,
            )
            || !custody.outcome.matches_candidate_resolution_request(
                &directory.directory_identity_digest,
                &request.normalized_name,
                &request.resolved_component_identity_digest,
                &request.expected_file_identity_digest,
                &request.concrete_servicing_generation_digest,
                &request.code_integrity_evidence_digest,
                &request.servicing_resolution_receipt_digest,
                &request.namespace_alias_currentness_receipt_digest,
            )
            || request
                .uses
                .iter()
                .any(|use_plan| !final_filesystem_use_matches(use_plan, request, resolution))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_FILESYSTEM_CHANGED");
        }
    }
    Ok(())
}

fn validate_owner_projection(
    plan: &WindowsRecursiveWaveDispatchPlanEvidence,
    closure: &SealedWindowsRecursiveResolutionClosure,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<()> {
    for owner in &plan.route_owners {
        let parsed_ordinal = match &owner.parse_disposition {
            WindowsRecursiveTargetParseDisposition::AlreadyParsed {
                parsed_image_ordinal,
            } => *parsed_image_ordinal,
            WindowsRecursiveTargetParseDisposition::NextFrontier {
                parse_receipt_ordinal,
                target_parse_wave_ordinal,
            } => {
                let receipt = closure
                    .parse_receipts
                    .get(*parse_receipt_ordinal)
                    .ok_or_else(|| {
                        anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_OWNER_PARSE_MISSING")
                    })?;
                if receipt.wave_ordinal != *target_parse_wave_ordinal
                    || receipt.producer_module_request_ordinal
                        != owner.earliest_producer_module_request_ordinal
                    || receipt.node != owner.target
                    || !owner.owner.matches_image_owner(&receipt.source_owner)
                    || receipt.source_owner_binding_digest
                        != owner.expected_source_owner_binding_digest
                    || receipt.image_material_identity_digest
                        != owner.expected_image_material_identity_digest
                {
                    bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_OWNER_PARSE_CHANGED");
                }
                receipt.parsed_image_ordinal
            }
        };
        let parsed = resolution
            .pe_import_graph
            .parsed_images
            .get(parsed_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_OWNER_MISSING"))?;
        let source_owner_matches = match parsed.source {
            WindowsPeParsedImageSource::BasePreleasePackage { .. } => {
                matches!(
                    (&owner.owner, &parsed.node),
                    (
                        WindowsRecursiveRouteOwnerRef::PackageContentLease {
                            package_file_ordinal: owner_package_ordinal,
                        },
                        WindowsLoaderModuleNode::PackageFile {
                            package_file_ordinal: parsed_package_ordinal,
                        },
                    ) if owner_package_ordinal == parsed_package_ordinal
                ) && parsed.source_binding_digest == owner.expected_source_owner_binding_digest
            }
            WindowsPeParsedImageSource::RecursiveExpansion {
                parse_receipt_ordinal,
            } => closure
                .parse_receipts
                .get(parse_receipt_ordinal)
                .is_some_and(|receipt| {
                    owner.owner.matches_image_owner(&receipt.source_owner)
                        && receipt.source_owner_binding_digest
                            == owner.expected_source_owner_binding_digest
                        && receipt.image_material_identity_digest
                            == owner.expected_image_material_identity_digest
                }),
        };
        if parsed.node != owner.target
            || parsed.image_material_identity_digest
                != owner.expected_image_material_identity_digest
            || !source_owner_matches
            || !final_owner_target_matches(owner, resolution)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FINAL_OWNER_CHANGED");
        }
    }
    Ok(())
}

fn final_owner_target_matches(
    owner: &WindowsRecursiveRouteOwnerPlanEntry,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let package = resolution.package_module_bindings.iter().find(|binding| {
        binding.module_request_ordinal == owner.earliest_producer_module_request_ordinal
    });
    let system = resolution.system_module_bindings.iter().find(|binding| {
        binding.module_request_ordinal == owner.earliest_producer_module_request_ordinal
    });
    if package.is_some() == system.is_some() {
        return false;
    }
    package.is_some_and(|binding| {
        owner.target
            == WindowsLoaderModuleNode::PackageFile {
                package_file_ordinal: binding.resolved_package_file_ordinal,
            }
            && owner.resolved_module_cache_key == binding.resolved_module_cache_key
    }) || system.is_some_and(|binding| {
        owner.target
            == crate::node_agent_compute_plugin_host::runtime_loader_load_set::pe_graph_validation::system_binding_target_node(binding)
            && owner.resolved_module_cache_key == binding.resolved_module_cache_key
    })
}

fn system_terminal_matches(
    terminal: &WindowsRecursiveModuleTerminalRef,
    binding: &WindowsLoaderSystemModuleBinding,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    match (&binding.resolution_origin, terminal) {
        (
            WindowsLoaderSystemResolutionOrigin::Preloaded {
                preloaded_module_ordinal: left,
            },
            WindowsRecursiveModuleTerminalRef::Direct {
                owner:
                    WindowsRecursiveRouteOwnerRef::AuthenticatedPreloadedModule {
                        preloaded_module_ordinal: right,
                    },
            },
        ) => left == right,
        (
            WindowsLoaderSystemResolutionOrigin::KnownDll {
                section_identity_digest,
            },
            WindowsRecursiveModuleTerminalRef::Direct {
                owner:
                    WindowsRecursiveRouteOwnerRef::KnownDllSection {
                        known_dll_authority_record_ordinal,
                    },
            },
        ) => resolution
            .known_dll_authority
            .sections
            .get(*known_dll_authority_record_ordinal)
            .is_some_and(|section| &section.section_identity_digest == section_identity_digest),
        (
            WindowsLoaderSystemResolutionOrigin::FilesystemSearch { .. },
            WindowsRecursiveModuleTerminalRef::Direct {
                owner:
                    WindowsRecursiveRouteOwnerRef::ResolvedFilesystemSystemImage {
                        resolution_request_ordinal,
                        route: WindowsRecursiveFilesystemUseRoute::OrdinaryFilesystem,
                    },
            },
        ) => binding
            .filesystem_image_ref
            .as_ref()
            .is_some_and(|reference| {
                reference.resolution_request_ordinal == *resolution_request_ordinal
            }),
        (
            WindowsLoaderSystemResolutionOrigin::SideBySide { .. },
            WindowsRecursiveModuleTerminalRef::Direct {
                owner:
                    WindowsRecursiveRouteOwnerRef::ResolvedFilesystemSystemImage {
                        resolution_request_ordinal,
                        route: WindowsRecursiveFilesystemUseRoute::SideBySide,
                    },
            },
        ) => binding
            .filesystem_image_ref
            .as_ref()
            .is_some_and(|reference| {
                reference.resolution_request_ordinal == *resolution_request_ordinal
            }),
        (
            WindowsLoaderSystemResolutionOrigin::ApiSet { .. },
            WindowsRecursiveModuleTerminalRef::ApiSetHost { .. },
        ) => api_set_terminal_matches(terminal, binding, resolution),
        _ => false,
    }
}

fn api_set_terminal_matches(
    terminal: &WindowsRecursiveModuleTerminalRef,
    binding: &WindowsLoaderSystemModuleBinding,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    let (
        WindowsLoaderSystemResolutionOrigin::ApiSet {
            normalized_contract_name,
            host_component_identity_digest,
            host_resolution,
        },
        WindowsRecursiveModuleTerminalRef::ApiSetHost {
            normalized_contract_name: planned_contract,
            normalized_host_module_cache_key: planned_host_cache_key,
            host_component_identity_digest: planned_host,
            host_owner,
            os_build_identity_digest,
            schema_identity_digest,
            contract_host_binding_set_digest,
            ..
        },
    ) = (&binding.resolution_origin, terminal)
    else {
        return false;
    };
    normalized_contract_name == planned_contract
        && host_component_identity_digest == planned_host
        && os_build_identity_digest == &resolution.api_set_authority.os_build_identity_digest
        && schema_identity_digest == &resolution.api_set_authority.schema_identity_digest
        && contract_host_binding_set_digest
            == &resolution
                .api_set_authority
                .contract_host_binding_set_digest
        && resolution
            .api_set_authority
            .contract_host_bindings
            .iter()
            .any(|contract| {
                contract.normalized_contract_name.as_str() == planned_contract.as_str()
                    && contract.host_module_cache_key.as_str() == planned_host_cache_key.as_str()
                    && contract.host_component_identity_digest.as_str() == planned_host.as_str()
                    && binding.resolved_component_identity_digest.as_str() == planned_host.as_str()
            })
        && api_set_host_resolution_matches(host_resolution, host_owner, binding, resolution)
}

fn api_set_host_resolution_matches(
    final_host: &WindowsLoaderApiSetHostResolution,
    planned: &WindowsRecursiveApiSetHostOwnerRef,
    binding: &WindowsLoaderSystemModuleBinding,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    match (final_host, planned) {
        (
            WindowsLoaderApiSetHostResolution::Preloaded {
                preloaded_module_ordinal: left,
            },
            WindowsRecursiveApiSetHostOwnerRef::AuthenticatedPreloadedModule {
                preloaded_module_ordinal: right,
            },
        ) => left == right,
        (
            WindowsLoaderApiSetHostResolution::KnownDll {
                section_identity_digest,
            },
            WindowsRecursiveApiSetHostOwnerRef::KnownDllSection {
                known_dll_authority_record_ordinal,
            },
        ) => resolution
            .known_dll_authority
            .sections
            .get(*known_dll_authority_record_ordinal)
            .is_some_and(|section| &section.section_identity_digest == section_identity_digest),
        (
            WindowsLoaderApiSetHostResolution::FilesystemSearch { .. },
            WindowsRecursiveApiSetHostOwnerRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal,
                route: WindowsRecursiveFilesystemUseRoute::OrdinaryFilesystem,
            },
        )
        | (
            WindowsLoaderApiSetHostResolution::SideBySide { .. },
            WindowsRecursiveApiSetHostOwnerRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal,
                route: WindowsRecursiveFilesystemUseRoute::SideBySide,
            },
        ) => binding
            .filesystem_image_ref
            .as_ref()
            .is_some_and(|reference| {
                reference.resolution_request_ordinal == *resolution_request_ordinal
            }),
        _ => false,
    }
}

fn final_filesystem_use_matches(
    use_plan: &WindowsRecursiveFilesystemImageUse,
    request: &WindowsRecursiveFilesystemImageRequestPlanEntry,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> bool {
    resolution.system_module_bindings.iter().any(|binding| {
        binding.module_request_ordinal == use_plan.module_request_ordinal
            && binding.resolved_component_identity_digest
                == request.resolved_component_identity_digest
            && binding
                .filesystem_image_ref
                .as_ref()
                .is_some_and(|reference| {
                    reference.resolution_request_ordinal == request.resolution_request_ordinal
                })
            && resolution
                .searched_names
                .get(use_plan.searched_name_ordinal)
                .is_some_and(|searched| {
                    searched.normalized_name == use_plan.normalized_name
                        && searched.search_directory_ordinal == use_plan.search_directory_ordinal
                        && searched.search_directory_authority_binding_digest
                            == use_plan.search_directory_authority_binding_digest
                })
    })
}

fn import_binding_module_request_ordinal(
    binding: &WindowsLoaderImportBindingRef,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Option<usize> {
    match binding {
        WindowsLoaderImportBindingRef::Package { binding_ordinal } => resolution
            .package_module_bindings
            .get(*binding_ordinal)
            .map(|binding| binding.module_request_ordinal),
        WindowsLoaderImportBindingRef::System { binding_ordinal } => resolution
            .system_module_bindings
            .get(*binding_ordinal)
            .map(|binding| binding.module_request_ordinal),
    }
}

fn final_import_kind_matches(
    planned: &WindowsRecursiveRequestImportKind,
    final_kind: WindowsLoaderImportEdgeKind,
) -> bool {
    matches!(
        (planned, final_kind),
        (
            WindowsRecursiveRequestImportKind::Normal,
            WindowsLoaderImportEdgeKind::NormalImport
        ) | (
            WindowsRecursiveRequestImportKind::Delay,
            WindowsLoaderImportEdgeKind::DelayImport
        ) | (
            WindowsRecursiveRequestImportKind::Forwarder,
            WindowsLoaderImportEdgeKind::Forwarder
        )
    )
}
