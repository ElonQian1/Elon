//! Borrow-only validation for recursive route-owner and terminal plan material.

use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    runtime_loader_load_set::system_resolution_validation::{
        canonical_loader_module_basename, normalized_loader_module_key_valid,
    },
};

use super::super::super::WindowsLoaderModuleNode;
use super::{custody::WindowsRecursiveResolutionAccumulatedCustody, plan::*, plan_digest};

pub(super) fn validate_route_owners(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<usize> {
    validate_base_parsed_image_owners(accumulated)?;
    let mut next_frontier_count = 0usize;
    let mut previous_earliest = None;
    for (ordinal, owner) in resolved.route_owners.iter().enumerate() {
        let earliest = resolved
            .module_resolutions
            .iter()
            .filter(|module| terminal_uses_owner(&module.terminal, &owner.owner))
            .map(|module| module.module_request_ordinal)
            .min();
        if owner.route_owner_ordinal != ordinal
            || earliest != Some(owner.earliest_producer_module_request_ordinal)
            || previous_earliest
                .is_some_and(|prior| prior >= owner.earliest_producer_module_request_ordinal)
            || resolved.route_owners[..ordinal]
                .iter()
                .any(|prior| prior.owner == owner.owner || prior.target == owner.target)
            || !normalized_loader_module_key_valid(&owner.resolved_module_cache_key)
            || owner_material_digest_invalid(owner)
            || owner.route_owner_binding_digest != plan_digest::route_owner_binding_digest(owner)?
            || !filesystem_owner_request_matches(owner, resolved)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ROUTE_OWNER_CHANGED");
        }
        validate_parse_disposition(accumulated, request, owner, &mut next_frontier_count)?;
        previous_earliest = Some(owner.earliest_producer_module_request_ordinal);
    }
    Ok(next_frontier_count)
}

pub(super) fn validate_base_parsed_image_owners(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
) -> Result<()> {
    let mut prelease_parsed_image_ordinals = HashSet::new();
    let mut package_file_ordinals = HashSet::new();
    let mut previous_postlease_parsed_image_ordinal = None;
    for owner in &accumulated.base_parsed_image_owners {
        let retained_package_lease_exists = accumulated
            .base_package_content_leases
            .iter()
            .any(|lease| lease.package_file_ordinal == owner.package_file_ordinal);
        if previous_postlease_parsed_image_ordinal
            .is_some_and(|previous| previous >= owner.postlease_parsed_image_ordinal)
            || !prelease_parsed_image_ordinals.insert(owner.prelease_parsed_image_ordinal)
            || !package_file_ordinals.insert(owner.package_file_ordinal)
            || !retained_package_lease_exists
            || !is_sha256(&owner.file_identity_digest)
            || !is_sha256(&owner.postlease_image_material_identity_digest)
            || !is_sha256(&owner.lease_generation_digest)
            || !is_sha256(&owner.source_owner_binding_digest)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_BASE_PARSED_OWNER_CHANGED");
        }
        previous_postlease_parsed_image_ordinal = Some(owner.postlease_parsed_image_ordinal);
    }
    Ok(())
}

pub(super) fn already_parsed_owner_matches(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    owner: &WindowsRecursiveRouteOwnerPlanEntry,
    parsed_image_ordinal: usize,
) -> bool {
    if let Some(base) = accumulated
        .base_parsed_image_owners
        .iter()
        .find(|base| base.postlease_parsed_image_ordinal == parsed_image_ordinal)
    {
        return base.postlease_parsed_image_ordinal == parsed_image_ordinal
            && matches!(
                &owner.owner,
                WindowsRecursiveRouteOwnerRef::PackageContentLease {
                    package_file_ordinal
                } if *package_file_ordinal == base.package_file_ordinal
            )
            && owner.target
                == (WindowsLoaderModuleNode::PackageFile {
                    package_file_ordinal: base.package_file_ordinal,
                })
            && owner.expected_source_owner_binding_digest == base.source_owner_binding_digest
            && owner.expected_image_material_identity_digest
                == base.postlease_image_material_identity_digest;
    }
    accumulated.completed_parse_receipts.iter().any(|receipt| {
        receipt.parsed_image_ordinal == parsed_image_ordinal
            && receipt.node == owner.target
            && owner.owner.matches_image_owner(&receipt.source_owner)
            && receipt.source_owner_binding_digest == owner.expected_source_owner_binding_digest
            && receipt.image_material_identity_digest
                == owner.expected_image_material_identity_digest
    })
}

pub(super) fn terminal_requires_search(terminal: &WindowsRecursiveModuleTerminalRef) -> bool {
    terminal_filesystem_request(terminal).is_some()
}

pub(super) fn terminal_shape_valid(terminal: &WindowsRecursiveModuleTerminalRef) -> Result<bool> {
    let WindowsRecursiveModuleTerminalRef::ApiSetHost {
        normalized_contract_name,
        normalized_host_module_cache_key,
        host_component_identity_digest,
        os_build_identity_digest,
        schema_identity_digest,
        contract_host_binding_set_digest,
        resolution_binding_digest,
        ..
    } = terminal
    else {
        return Ok(true);
    };
    Ok(
        canonical_loader_module_basename(normalized_contract_name).as_deref()
            == Some(normalized_contract_name.as_str())
            && normalized_loader_module_key_valid(normalized_host_module_cache_key)
            && [
                host_component_identity_digest,
                os_build_identity_digest,
                schema_identity_digest,
                contract_host_binding_set_digest,
                resolution_binding_digest,
            ]
            .into_iter()
            .all(|digest| is_sha256(digest))
            && plan_digest::api_set_terminal_binding_digest(terminal)?.as_deref()
                == Some(resolution_binding_digest.as_str()),
    )
}

pub(super) fn terminal_uses_owner(
    terminal: &WindowsRecursiveModuleTerminalRef,
    owner: &WindowsRecursiveRouteOwnerRef,
) -> bool {
    match terminal {
        WindowsRecursiveModuleTerminalRef::Direct { owner: terminal } => terminal == owner,
        WindowsRecursiveModuleTerminalRef::ApiSetHost { host_owner, .. } => {
            api_set_host_matches_owner(host_owner, owner)
        }
    }
}

pub(super) fn terminal_filesystem_request(
    terminal: &WindowsRecursiveModuleTerminalRef,
) -> Option<(usize, &WindowsRecursiveFilesystemUseRoute)> {
    match terminal {
        WindowsRecursiveModuleTerminalRef::Direct {
            owner:
                WindowsRecursiveRouteOwnerRef::ResolvedFilesystemSystemImage {
                    resolution_request_ordinal,
                    route,
                },
        } => Some((*resolution_request_ordinal, route)),
        WindowsRecursiveModuleTerminalRef::ApiSetHost {
            host_owner:
                WindowsRecursiveApiSetHostOwnerRef::ResolvedFilesystemSystemImage {
                    resolution_request_ordinal,
                    route,
                },
            ..
        } => Some((*resolution_request_ordinal, route)),
        _ => None,
    }
}

pub(super) fn terminal_uses_filesystem_request(
    terminal: &WindowsRecursiveModuleTerminalRef,
    resolution_request_ordinal: usize,
    route: &WindowsRecursiveFilesystemUseRoute,
) -> bool {
    terminal_filesystem_request(terminal).is_some_and(|(ordinal, terminal_route)| {
        ordinal == resolution_request_ordinal && terminal_route == route
    })
}

fn validate_parse_disposition(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
    owner: &WindowsRecursiveRouteOwnerPlanEntry,
    next_frontier_count: &mut usize,
) -> Result<()> {
    match &owner.parse_disposition {
        WindowsRecursiveTargetParseDisposition::AlreadyParsed {
            parsed_image_ordinal,
        } => {
            if !already_parsed_owner_matches(accumulated, owner, *parsed_image_ordinal) {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_ALREADY_PARSED_TARGET_CHANGED");
            }
        }
        WindowsRecursiveTargetParseDisposition::NextFrontier {
            parse_receipt_ordinal,
            target_parse_wave_ordinal,
        } => {
            let expected_parse_ordinal = accumulated
                .completed_parse_receipts
                .len()
                .checked_add(*next_frontier_count)
                .ok_or_else(count_overflow)?;
            if *parse_receipt_ordinal != expected_parse_ordinal
                || request.producer_wave_ordinal.checked_add(1) != Some(*target_parse_wave_ordinal)
                || accumulated
                    .completed_parse_receipts
                    .iter()
                    .any(|receipt| receipt.node == owner.target)
                || matches!(
                    &owner.owner,
                    WindowsRecursiveRouteOwnerRef::PackageContentLease { .. }
                )
            {
                bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_NEXT_FRONTIER_TARGET_CHANGED");
            }
            *next_frontier_count = next_frontier_count
                .checked_add(1)
                .ok_or_else(count_overflow)?;
        }
    }
    Ok(())
}

fn filesystem_owner_request_matches(
    owner: &WindowsRecursiveRouteOwnerPlanEntry,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> bool {
    match &owner.owner {
        WindowsRecursiveRouteOwnerRef::ResolvedFilesystemSystemImage {
            resolution_request_ordinal,
            route,
        } => resolved
            .filesystem_image_requests
            .iter()
            .find(|request| request.resolution_request_ordinal == *resolution_request_ordinal)
            .is_some_and(|request| request.uses.iter().all(|use_plan| &use_plan.route == route)),
        _ => true,
    }
}

fn api_set_host_matches_owner(
    host: &WindowsRecursiveApiSetHostOwnerRef,
    owner: &WindowsRecursiveRouteOwnerRef,
) -> bool {
    match (host, owner) {
        (
            WindowsRecursiveApiSetHostOwnerRef::AuthenticatedPreloadedModule {
                preloaded_module_ordinal: left,
            },
            WindowsRecursiveRouteOwnerRef::AuthenticatedPreloadedModule {
                preloaded_module_ordinal: right,
            },
        ) => left == right,
        (
            WindowsRecursiveApiSetHostOwnerRef::KnownDllSection {
                known_dll_authority_record_ordinal: left,
            },
            WindowsRecursiveRouteOwnerRef::KnownDllSection {
                known_dll_authority_record_ordinal: right,
            },
        ) => left == right,
        (
            WindowsRecursiveApiSetHostOwnerRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal: left,
                route: left_route,
            },
            WindowsRecursiveRouteOwnerRef::ResolvedFilesystemSystemImage {
                resolution_request_ordinal: right,
                route: right_route,
            },
        ) => left == right && left_route == right_route,
        _ => false,
    }
}

fn owner_material_digest_invalid(owner: &WindowsRecursiveRouteOwnerPlanEntry) -> bool {
    [
        &owner.expected_source_owner_binding_digest,
        &owner.expected_image_material_identity_digest,
        &owner.route_owner_binding_digest,
    ]
    .into_iter()
    .any(|digest| !is_sha256(digest))
}

fn count_overflow() -> anyhow::Error {
    anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_PRE_DISPATCH_COUNT_OVERFLOW")
}
