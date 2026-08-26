//! Borrow-only fail-closed validation for one recursive wave before its first dispatch.

use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    runtime_loader_load_set::{
        launch_path_discovery::symbol_is_exact,
        system_resolution_validation::canonical_loader_module_basename,
    },
};

use super::super::super::WindowsPostLeaseModuleEdgeLocator;
use super::{
    custody::WindowsRecursiveResolutionAccumulatedCustody, plan::*, plan_digest,
    plan_forwarder_validation, plan_owner_validation,
};

pub(super) struct WindowsRecursiveWaveDerivedProjection {
    pub(super) recursive_wave_count: usize,
    pub(super) parsed_image_count: usize,
    pub(super) module_request_count: usize,
    pub(super) searched_name_count: usize,
    pub(super) system_image_request_count: usize,
    pub(super) forwarder_hop_depth: usize,
}

type FilesystemDedupeKey<'plan> = (
    &'plan str,
    &'plan str,
    &'plan str,
    &'plan str,
    &'plan str,
    &'plan str,
    &'plan str,
    u8,
);

pub(super) fn validate_whole_before_first_dispatch(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<WindowsRecursiveWaveDerivedProjection> {
    validate_accumulated_prefix(accumulated, request)?;
    validate_request_plan(accumulated, request)?;
    let next_frontier_count = validate_resolved_plan(accumulated, request, resolved)?;
    let forwarder_hop_depth = plan_forwarder_validation::validate_cumulative_forwarder_chains(
        accumulated,
        request,
        resolved,
    )?;

    let recursive_wave_count = request
        .producer_wave_ordinal
        .checked_add(usize::from(next_frontier_count > 0))
        .ok_or_else(count_overflow)?;
    let parsed_image_count = accumulated
        .base_parsed_image_owners
        .len()
        .checked_add(accumulated.completed_parse_receipts.len())
        .and_then(|count| count.checked_add(next_frontier_count))
        .ok_or_else(count_overflow)?;
    Ok(WindowsRecursiveWaveDerivedProjection {
        recursive_wave_count,
        parsed_image_count,
        module_request_count: checked_end(
            request.first_module_request_ordinal,
            request.module_requests.len(),
        )?,
        searched_name_count: checked_end(
            request.first_searched_name_ordinal,
            resolved.searched_name_dispositions.len(),
        )?,
        system_image_request_count: checked_end(
            request.first_system_image_request_ordinal,
            resolved.filesystem_image_requests.len(),
        )?,
        forwarder_hop_depth,
    })
}

fn validate_accumulated_prefix(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
) -> Result<()> {
    plan_owner_validation::validate_base_parsed_image_owners(accumulated)?;
    let Some(previous) = accumulated.completed_acquisition_receipts.last() else {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PRE_DISPATCH_A0_MISSING");
    };
    let base_owner_set_digest =
        plan_digest::base_parsed_image_owner_set_digest(&accumulated.base_parsed_image_owners)?;
    let forwarder_chain_set_digest =
        plan_digest::retained_forwarder_chain_set_digest(&accumulated.retained_forwarder_chains)?;
    let expected_source_frontier = previous
        .next_frontier_parse_receipt_ordinals
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let actual_source_frontier = request
        .source_frontier
        .iter()
        .map(|source| source.parse_receipt_ordinal)
        .collect::<Vec<_>>();
    if request.producer_wave_ordinal == 0
        || request.producer_wave_ordinal != accumulated.completed_acquisition_receipts.len()
        || previous.acquisition_receipt_ordinal.checked_add(1)
            != Some(request.producer_wave_ordinal)
        || previous.target_parse_wave_ordinal != Some(request.producer_wave_ordinal)
        || request.previous_acquisition_receipt_digest != previous.receipt_digest
        || request.input_custody_digest != previous.output_custody_digest
        || accumulated.whole_state_digest != request.input_custody_digest
        || previous.base_parsed_image_owner_set_digest != base_owner_set_digest
        || previous.retained_forwarder_chain_set_digest != forwarder_chain_set_digest
        || expected_source_frontier.is_empty()
        || actual_source_frontier != expected_source_frontier
        || request.first_module_request_ordinal
            != checked_end(
                previous.first_module_request_ordinal,
                previous.module_request_count,
            )?
        || request.first_searched_name_ordinal
            != checked_end(
                previous.first_searched_name_ordinal,
                previous.searched_name_count,
            )?
        || request.first_system_image_request_ordinal
            != checked_end(
                previous.first_system_image_request_ordinal,
                previous.system_image_request_count,
            )?
        || request.authenticated_recursive_policy_digest
            != accumulated.authenticated_policy.digest()
        || request.parser_policy_digest != accumulated.authenticated_policy.parser_policy_digest()
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PRE_DISPATCH_PREFIX_CHANGED");
    }
    Ok(())
}

fn validate_request_plan(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
) -> Result<()> {
    validate_request_frontier(accumulated, request)?;
    validate_request_edges(request)
}

fn validate_request_frontier(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
) -> Result<()> {
    if request.source_request_plan_digest != plan_digest::request_plan_digest(request)?
        || !is_sha256(&request.source_request_plan_digest)
        || !is_sha256(&request.previous_acquisition_receipt_digest)
        || !is_sha256(&request.input_custody_digest)
        || !is_sha256(&request.authenticated_recursive_policy_digest)
        || !is_sha256(&request.parser_policy_digest)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_REQUEST_PLAN_DIGEST_CHANGED");
    }
    for (position, source) in request.source_frontier.iter().enumerate() {
        let receipt = accumulated
            .completed_parse_receipts
            .get(source.parse_receipt_ordinal)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SOURCE_RECEIPT_MISSING"))?;
        if source.parse_receipt_ordinal != request.source_frontier[position].parse_receipt_ordinal
            || position > 0
                && request.source_frontier[position - 1].parse_receipt_ordinal
                    >= source.parse_receipt_ordinal
            || !source.matches_receipt(receipt)
            || source.wave_ordinal != request.producer_wave_ordinal
            || source.producer_acquisition_receipt_ordinal.checked_add(1)
                != Some(source.wave_ordinal)
            || source.parser_policy_digest != request.parser_policy_digest
            || [
                &source.receipt_digest,
                &source.source_owner_binding_digest,
                &source.image_material_identity_digest,
                &source.parser_policy_digest,
                &source.import_table_digest,
            ]
            .into_iter()
            .any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SOURCE_FRONTIER_CHANGED");
        }
    }
    Ok(())
}

fn validate_request_edges(request: &WindowsRecursiveWaveRequestPlan) -> Result<()> {
    let mut next_local_edges = vec![0usize; request.source_frontier.len()];
    let mut import_kind_counts = vec![[0usize; 3]; request.source_frontier.len()];
    let mut prior_local_keys = vec![None; request.source_frontier.len()];
    let mut prior_source_position = 0usize;
    for (offset, edge) in request.module_requests.iter().enumerate() {
        let source_position = request
            .source_frontier
            .binary_search_by_key(&edge.source_parse_receipt_ordinal, |source| {
                source.parse_receipt_ordinal
            })
            .map_err(|_| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_SOURCE_MISSING"))?;
        let source = &request.source_frontier[source_position];
        let local_key = local_edge_key(edge)?;
        if edge.module_request_ordinal
            != request
                .first_module_request_ordinal
                .checked_add(offset)
                .ok_or_else(count_overflow)?
            || edge.global_import_edge_ordinal != edge.module_request_ordinal
            || source_position < prior_source_position
            || edge.importer_graph_edge_ordinal != next_local_edges[source_position]
            || edge.importer_parsed_image_ordinal != source.parsed_image_ordinal
            || edge.importer != source.node
            || canonical_loader_module_basename(&edge.normalized_requested_name).as_deref()
                != Some(edge.normalized_requested_name.as_str())
            || !symbol_is_exact(
                edge.imported_symbol_name.as_deref(),
                edge.imported_symbol_ordinal,
            )
            || !strictly_increasing(&edge.ordered_search_step_ordinals)
            || !is_sha256(&edge.edge_request_binding_digest)
            || edge.edge_request_binding_digest != plan_digest::edge_request_binding_digest(edge)?
            || !edge_locator_matches_kind(edge)?
            || prior_local_keys[source_position].is_some_and(|prior| prior >= local_key)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_REQUEST_CHANGED");
        }
        let kind_position = import_kind_position(&edge.import_kind);
        import_kind_counts[source_position][kind_position] = import_kind_counts[source_position]
            [kind_position]
            .checked_add(1)
            .ok_or_else(count_overflow)?;
        next_local_edges[source_position] = next_local_edges[source_position]
            .checked_add(1)
            .ok_or_else(count_overflow)?;
        prior_local_keys[source_position] = Some(local_key);
        prior_source_position = source_position;
    }
    for (position, source) in request.source_frontier.iter().enumerate() {
        let expected = source
            .normal_import_count
            .checked_add(source.delay_import_count)
            .and_then(|count| count.checked_add(source.forwarder_count))
            .ok_or_else(count_overflow)?;
        let expected_by_kind = [
            source.normal_import_count,
            source.delay_import_count,
            source.forwarder_count,
        ];
        if next_local_edges[position] != expected
            || import_kind_counts[position] != expected_by_kind
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_EDGE_COVERAGE_CHANGED");
        }
    }
    Ok(())
}

fn validate_resolved_plan(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<usize> {
    validate_resolution_digests(request, resolved)?;
    validate_retained_directories(accumulated)?;
    if resolved.producer_wave_ordinal != request.producer_wave_ordinal
        || resolved.authenticated_recursive_policy_digest
            != request.authenticated_recursive_policy_digest
        || resolved.parser_policy_digest != request.parser_policy_digest
        || resolved.source_request_plan_digest != request.source_request_plan_digest
        || resolved.module_resolutions.len() != request.module_requests.len()
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_RESOLUTION_PLAN_SOURCE_CHANGED");
    }
    validate_modules_and_searches(accumulated, request, resolved)?;
    validate_filesystem_requests(request, resolved)?;
    plan_owner_validation::validate_route_owners(accumulated, request, resolved)
}

fn validate_resolution_digests(
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<()> {
    if resolved.terminal_resolution_set_digest
        != plan_digest::terminal_resolution_set_digest(
            request.producer_wave_ordinal,
            &resolved.module_resolutions,
        )?
        || resolved.searched_name_disposition_set_digest
            != plan_digest::searched_name_disposition_set_digest(
                request.producer_wave_ordinal,
                &resolved.searched_name_dispositions,
            )?
        || resolved.filesystem_request_set_digest
            != plan_digest::filesystem_request_set_digest(
                request.producer_wave_ordinal,
                &resolved.filesystem_image_requests,
            )?
        || resolved.route_owner_set_digest
            != plan_digest::route_owner_set_digest(
                request.producer_wave_ordinal,
                &resolved.route_owners,
            )?
        || resolved.resolved_plan_digest != plan_digest::resolved_plan_digest(resolved)?
        || [
            &resolved.authenticated_recursive_policy_digest,
            &resolved.parser_policy_digest,
            &resolved.source_request_plan_digest,
            &resolved.terminal_resolution_set_digest,
            &resolved.searched_name_disposition_set_digest,
            &resolved.filesystem_request_set_digest,
            &resolved.route_owner_set_digest,
            &resolved.resolved_plan_digest,
        ]
        .into_iter()
        .any(|digest| !is_sha256(digest))
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_RESOLUTION_PLAN_DIGEST_CHANGED");
    }
    Ok(())
}

fn validate_modules_and_searches(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<()> {
    let mut used_searches = HashSet::new();
    for (offset, (module, edge)) in resolved
        .module_resolutions
        .iter()
        .zip(&request.module_requests)
        .enumerate()
    {
        validate_module_searches(
            accumulated,
            request,
            resolved,
            offset,
            module,
            edge,
            &mut used_searches,
        )?;
    }
    if used_searches.len() != resolved.searched_name_dispositions.len()
        || resolved
            .searched_name_dispositions
            .iter()
            .enumerate()
            .any(|(offset, searched)| {
                request.first_searched_name_ordinal.checked_add(offset)
                    != Some(searched.searched_name_ordinal)
                    || !used_searches.contains(&searched.searched_name_ordinal)
            })
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SEARCH_COVERAGE_CHANGED");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_module_searches(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
    offset: usize,
    module: &WindowsRecursiveModuleResolutionPlanEntry,
    edge: &WindowsRecursiveParsedEdgeRequest,
    used_searches: &mut HashSet<usize>,
) -> Result<()> {
    let needs_search = plan_owner_validation::terminal_requires_search(&module.terminal);
    let expected_search_name = expected_module_search_name(&module.terminal, edge)?;
    if module.module_request_ordinal != edge.module_request_ordinal
        || request.first_module_request_ordinal.checked_add(offset)
            != Some(module.module_request_ordinal)
        || module.resolution_binding_digest
            != plan_digest::module_resolution_binding_digest(module)?
        || !is_sha256(&module.resolution_binding_digest)
        || module.searched_name_ordinals.len() > edge.ordered_search_step_ordinals.len()
        || (needs_search && module.searched_name_ordinals.is_empty())
        || (!needs_search && !module.searched_name_ordinals.is_empty())
        || !plan_owner_validation::terminal_shape_valid(&module.terminal)?
        || !resolved
            .route_owners
            .iter()
            .any(|owner| plan_owner_validation::terminal_uses_owner(&module.terminal, &owner.owner))
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_MODULE_RESOLUTION_CHANGED");
    }
    for (position, searched_ordinal) in module.searched_name_ordinals.iter().enumerate() {
        let searched = resolved
            .searched_name_dispositions
            .get(
                searched_ordinal
                    .checked_sub(request.first_searched_name_ordinal)
                    .ok_or_else(count_overflow)?,
            )
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SEARCH_MISSING"))?;
        let terminal_position =
            position.checked_add(1) == Some(module.searched_name_ordinals.len());
        let disposition_matches = if terminal_position {
            matches!(
                &searched.disposition,
                WindowsRecursiveSearchedNameDisposition::Terminal { terminal }
                    if terminal == &module.terminal
            )
        } else {
            matches!(
                &searched.disposition,
                WindowsRecursiveSearchedNameDisposition::MustRemainAbsent
            )
        };
        if !used_searches.insert(*searched_ordinal)
            || searched.searched_name_ordinal != *searched_ordinal
            || searched.module_request_ordinal != module.module_request_ordinal
            || searched.step_position != position
            || edge.ordered_search_step_ordinals.get(position)
                != Some(&searched.search_directory_ordinal)
            || searched.normalized_name != expected_search_name
            || !disposition_matches
            || !search_directory_matches(accumulated, searched)
            || searched.disposition_binding_digest
                != plan_digest::searched_name_disposition_binding_digest(searched)?
            || searched.grant_request_digest
                != plan_digest::searched_name_grant_request_digest(searched)?
            || !is_sha256(&searched.disposition_binding_digest)
            || !is_sha256(&searched.grant_request_digest)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_SEARCH_DISPOSITION_CHANGED");
        }
    }
    Ok(())
}

fn expected_module_search_name<'plan>(
    terminal: &'plan WindowsRecursiveModuleTerminalRef,
    edge: &'plan WindowsRecursiveParsedEdgeRequest,
) -> Result<&'plan str> {
    match terminal {
        WindowsRecursiveModuleTerminalRef::ApiSetHost {
            normalized_contract_name,
            normalized_host_module_cache_key,
            ..
        } => {
            if normalized_contract_name != &edge.normalized_requested_name {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_API_SET_CONTRACT_CHANGED");
            }
            Ok(normalized_host_module_cache_key)
        }
        WindowsRecursiveModuleTerminalRef::Direct { .. } => Ok(&edge.normalized_requested_name),
    }
}

fn validate_filesystem_requests(
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<()> {
    let mut dedupe_keys = HashSet::new();
    let mut used_modules = HashSet::new();
    let mut previous_earliest_use = None;
    for (offset, filesystem) in resolved.filesystem_image_requests.iter().enumerate() {
        let ordinal = request
            .first_system_image_request_ordinal
            .checked_add(offset)
            .ok_or_else(count_overflow)?;
        let earliest_use =
            validate_filesystem_request_shape(filesystem, ordinal, &mut dedupe_keys)?;
        if previous_earliest_use.is_some_and(|prior| prior >= earliest_use) {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FILESYSTEM_ORDER_CHANGED");
        }
        validate_filesystem_uses(filesystem, resolved, &mut used_modules)?;
        previous_earliest_use = Some(earliest_use);
    }
    validate_filesystem_terminal_coverage(resolved)
}

fn validate_filesystem_request_shape<'plan>(
    filesystem: &'plan WindowsRecursiveFilesystemImageRequestPlanEntry,
    ordinal: usize,
    dedupe_keys: &mut HashSet<FilesystemDedupeKey<'plan>>,
) -> Result<(usize, usize, usize)> {
    let Some(primary) = filesystem.uses.first() else {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FILESYSTEM_REQUEST_CHANGED");
    };
    let route = filesystem_route_ordinal(&primary.route);
    let dedupe_key = (
        filesystem.normalized_name.as_str(),
        filesystem
            .search_directory_authority_binding_digest
            .as_str(),
        filesystem.resolved_component_identity_digest.as_str(),
        filesystem.expected_file_identity_digest.as_str(),
        filesystem.concrete_servicing_generation_digest.as_str(),
        filesystem.code_integrity_evidence_digest.as_str(),
        filesystem.candidate_binding_digest.as_str(),
        route,
    );
    if filesystem.resolution_request_ordinal != ordinal
        || filesystem.canonical_dedupe_ordinal != ordinal
        || filesystem.primary_use_ordinal != 0
        || primary.normalized_name != filesystem.normalized_name
        || primary.search_directory_authority_binding_digest
            != filesystem.search_directory_authority_binding_digest
        || !dedupe_keys.insert(dedupe_key)
        || filesystem.lease_request_digest
            != plan_digest::filesystem_lease_request_digest(filesystem)?
        || filesystem_request_digest_changed(filesystem)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FILESYSTEM_REQUEST_CHANGED");
    }
    Ok(filesystem_use_key(primary))
}

fn validate_filesystem_uses(
    filesystem: &WindowsRecursiveFilesystemImageRequestPlanEntry,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
    used_modules: &mut HashSet<usize>,
) -> Result<()> {
    let primary_route = &filesystem.uses[filesystem.primary_use_ordinal].route;
    let mut prior_use_key = None;
    for use_plan in &filesystem.uses {
        let use_key = filesystem_use_key(use_plan);
        if prior_use_key.is_some_and(|prior| prior >= use_key)
            || &use_plan.route != primary_route
            || !used_modules.insert(use_plan.module_request_ordinal)
            || !filesystem_use_matches(use_plan, filesystem, resolved)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FILESYSTEM_USE_CHANGED");
        }
        prior_use_key = Some(use_key);
    }
    Ok(())
}

fn validate_filesystem_terminal_coverage(
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<()> {
    for module in &resolved.module_resolutions {
        let Some((request_ordinal, route)) =
            plan_owner_validation::terminal_filesystem_request(&module.terminal)
        else {
            continue;
        };
        let exact_use_count = resolved
            .filesystem_image_requests
            .iter()
            .filter(|request| request.resolution_request_ordinal == request_ordinal)
            .flat_map(|request| &request.uses)
            .filter(|use_plan| {
                use_plan.module_request_ordinal == module.module_request_ordinal
                    && &use_plan.route == route
            })
            .count();
        if exact_use_count != 1 {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FILESYSTEM_TERMINAL_COVERAGE_CHANGED");
        }
    }
    Ok(())
}

fn validate_retained_directories(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
) -> Result<()> {
    if accumulated
        .retained_search_directories
        .iter()
        .enumerate()
        .any(|(ordinal, directory)| {
            directory.search_directory_ordinal != ordinal
                || !is_sha256(&directory.directory_identity_digest)
                || !is_sha256(&directory.authority_binding_digest)
        })
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_RETAINED_DIRECTORY_CHANGED");
    }
    Ok(())
}

fn search_directory_matches(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    searched: &WindowsRecursiveSearchedNamePlanEntry,
) -> bool {
    accumulated
        .retained_search_directories
        .get(searched.search_directory_ordinal)
        .is_some_and(|directory| {
            directory.search_directory_ordinal == searched.search_directory_ordinal
                && directory.authority_binding_digest
                    == searched.search_directory_authority_binding_digest
        })
}

fn filesystem_use_matches(
    use_plan: &WindowsRecursiveFilesystemImageUse,
    filesystem: &WindowsRecursiveFilesystemImageRequestPlanEntry,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> bool {
    let Some(module) = resolved
        .module_resolutions
        .iter()
        .find(|module| module.module_request_ordinal == use_plan.module_request_ordinal)
    else {
        return false;
    };
    let Some(searched) = resolved
        .searched_name_dispositions
        .iter()
        .find(|searched| searched.searched_name_ordinal == use_plan.searched_name_ordinal)
    else {
        return false;
    };
    use_plan.normalized_name == filesystem.normalized_name
        && use_plan.search_directory_authority_binding_digest
            == filesystem.search_directory_authority_binding_digest
        && searched.module_request_ordinal == module.module_request_ordinal
        && searched.search_directory_ordinal == use_plan.search_directory_ordinal
        && searched.normalized_name == use_plan.normalized_name
        && searched.search_directory_authority_binding_digest
            == use_plan.search_directory_authority_binding_digest
        && matches!(
            &searched.disposition,
            WindowsRecursiveSearchedNameDisposition::Terminal { terminal }
                if terminal == &module.terminal
        )
        && plan_owner_validation::terminal_uses_filesystem_request(
            &module.terminal,
            filesystem.resolution_request_ordinal,
            &use_plan.route,
        )
}

fn filesystem_request_digest_changed(
    filesystem: &WindowsRecursiveFilesystemImageRequestPlanEntry,
) -> bool {
    [
        &filesystem.search_directory_authority_binding_digest,
        &filesystem.resolved_component_identity_digest,
        &filesystem.expected_file_identity_digest,
        &filesystem.concrete_servicing_generation_digest,
        &filesystem.code_integrity_evidence_digest,
        &filesystem.servicing_resolution_receipt_digest,
        &filesystem.namespace_alias_currentness_receipt_digest,
        &filesystem.candidate_binding_digest,
        &filesystem.lease_request_digest,
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
}

fn filesystem_use_key(use_plan: &WindowsRecursiveFilesystemImageUse) -> (usize, usize, usize) {
    (
        use_plan.module_request_ordinal,
        use_plan.searched_name_ordinal,
        use_plan.search_directory_ordinal,
    )
}

fn filesystem_route_ordinal(route: &WindowsRecursiveFilesystemUseRoute) -> u8 {
    match route {
        WindowsRecursiveFilesystemUseRoute::OrdinaryFilesystem => 0,
        WindowsRecursiveFilesystemUseRoute::SideBySide => 1,
    }
}

fn edge_locator_matches_kind(edge: &WindowsRecursiveParsedEdgeRequest) -> Result<bool> {
    Ok(match (&edge.import_kind, &edge.edge_locator) {
        (
            WindowsRecursiveRequestImportKind::Normal | WindowsRecursiveRequestImportKind::Delay,
            WindowsPostLeaseModuleEdgeLocator::Import {
                source_import_edge_ordinal,
                edge_evidence_digest,
                ..
            },
        ) => {
            *source_import_edge_ordinal == edge.global_import_edge_ordinal
                && is_sha256(edge_evidence_digest)
        }
        (
            WindowsRecursiveRequestImportKind::Forwarder,
            WindowsPostLeaseModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                hop_evidence_digest,
                ..
            },
        ) => {
            *source_import_edge_ordinal < edge.global_import_edge_ordinal
                && is_sha256(hop_evidence_digest)
        }
        _ => false,
    })
}

fn local_edge_key(edge: &WindowsRecursiveParsedEdgeRequest) -> Result<(u8, usize, usize)> {
    match (&edge.import_kind, &edge.edge_locator) {
        (
            WindowsRecursiveRequestImportKind::Normal,
            WindowsPostLeaseModuleEdgeLocator::Import {
                descriptor_ordinal,
                thunk_ordinal,
                ..
            },
        ) => Ok((0, *descriptor_ordinal, *thunk_ordinal)),
        (
            WindowsRecursiveRequestImportKind::Delay,
            WindowsPostLeaseModuleEdgeLocator::Import {
                descriptor_ordinal,
                thunk_ordinal,
                ..
            },
        ) => Ok((1, *descriptor_ordinal, *thunk_ordinal)),
        (
            WindowsRecursiveRequestImportKind::Forwarder,
            WindowsPostLeaseModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                forwarder_hop_ordinal,
                ..
            },
        ) => Ok((2, *source_import_edge_ordinal, *forwarder_hop_ordinal)),
        _ => bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_IMPORTER_EDGE_LOCATOR_CHANGED"),
    }
}

fn import_kind_position(kind: &WindowsRecursiveRequestImportKind) -> usize {
    match kind {
        WindowsRecursiveRequestImportKind::Normal => 0,
        WindowsRecursiveRequestImportKind::Delay => 1,
        WindowsRecursiveRequestImportKind::Forwarder => 2,
    }
}

fn strictly_increasing(values: &[usize]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn checked_end(first: usize, count: usize) -> Result<usize> {
    first.checked_add(count).ok_or_else(count_overflow)
}

fn count_overflow() -> anyhow::Error {
    anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PRE_DISPATCH_COUNT_OVERFLOW")
}
