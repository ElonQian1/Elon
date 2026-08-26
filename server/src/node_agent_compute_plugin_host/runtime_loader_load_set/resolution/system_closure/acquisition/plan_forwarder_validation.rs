//! Exact cumulative forwarder-chain validation before one recursive wave can dispatch.

use anyhow::{anyhow, bail, Result};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256,
    runtime_loader_load_set::launch_path_discovery::{
        symbol_is_exact, WindowsPreliminaryModuleEdgeLocator,
    },
};

use super::super::super::{
    SealedWindowsLoaderResolutionAuthority, WindowsLoaderImportEdgeKind,
    WindowsLoaderModuleEdgeLocator, WindowsLoaderModuleNode, WindowsPostLeaseModuleEdgeLocator,
};
use super::{
    custody::WindowsRecursiveResolutionAccumulatedCustody, plan::*, plan_digest,
    plan_owner_validation,
};

#[derive(Clone, PartialEq, Eq)]
struct WorkingForwarderSymbol {
    name: Option<String>,
    ordinal: Option<u16>,
}

#[derive(Clone, PartialEq, Eq)]
struct WorkingForwarderTarget {
    node: WindowsLoaderModuleNode,
    symbol: WorkingForwarderSymbol,
}

struct WorkingForwarderChain {
    source_import_edge_ordinal: usize,
    next_hop_ordinal: usize,
    current_target: WorkingForwarderTarget,
    visited_targets: Vec<WorkingForwarderTarget>,
}

struct FinalForwarderEdge<'edge> {
    module_request_ordinal: usize,
    global_import_edge_ordinal: usize,
    locator: &'edge WindowsLoaderModuleEdgeLocator,
    importer: &'edge WindowsLoaderModuleNode,
    edge_kind: WindowsLoaderImportEdgeKind,
    imported_symbol_name: Option<&'edge str>,
    imported_symbol_ordinal: Option<u16>,
    target: WindowsLoaderModuleNode,
}

pub(super) fn validate_cumulative_forwarder_chains(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
) -> Result<usize> {
    let mut chains = validate_retained_chains(accumulated, request)?;
    for edge in &request.module_requests {
        let target = resolved_target(resolved, edge.module_request_ordinal)?;
        let target = WorkingForwarderTarget {
            node: target.clone(),
            symbol: exact_symbol(
                edge.imported_symbol_name.as_deref(),
                edge.imported_symbol_ordinal,
            )?,
        };
        apply_current_edge(&mut chains, edge, target)?;
    }
    Ok(chains
        .iter()
        .map(|chain| chain.next_hop_ordinal)
        .max()
        .unwrap_or(0))
}

pub(super) fn final_forwarder_chain_set_digest_through(
    module_request_end: usize,
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Result<String> {
    let mut edges = final_forwarder_edges(resolution)
        .into_iter()
        .filter(|edge| edge.module_request_ordinal < module_request_end)
        .collect::<Vec<_>>();
    edges.sort_by_key(|edge| edge.module_request_ordinal);
    if edges.len() != module_request_end {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_PREFIX_CHANGED");
    }
    let mut chains = Vec::new();
    for (ordinal, edge) in edges.iter().enumerate() {
        if edge.module_request_ordinal != ordinal
            || edge.global_import_edge_ordinal != edge.module_request_ordinal
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_PREFIX_CHANGED");
        }
        let target = final_target(edge)?;
        apply_final_edge(&mut chains, edge, target)?;
    }
    let entries = retained_entries(chains)?;
    plan_digest::retained_forwarder_chain_set_digest(&entries)
}

fn validate_retained_chains(
    accumulated: &WindowsRecursiveResolutionAccumulatedCustody<'_>,
    request: &WindowsRecursiveWaveRequestPlan,
) -> Result<Vec<WorkingForwarderChain>> {
    let mut chains = Vec::with_capacity(accumulated.retained_forwarder_chains.len());
    let mut prior_root = None;
    for retained in &accumulated.retained_forwarder_chains {
        if retained.source_import_edge_ordinal >= request.first_module_request_ordinal
            || prior_root.is_some_and(|prior| prior >= retained.source_import_edge_ordinal)
            || !is_sha256(&retained.chain_binding_digest)
            || retained.chain_binding_digest
                != plan_digest::retained_forwarder_chain_binding_digest(retained)?
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_RETAINED_FORWARDER_CHAIN_CHANGED");
        }
        chains.push(validate_retained_chain(retained)?);
        prior_root = Some(retained.source_import_edge_ordinal);
    }
    Ok(chains)
}

fn validate_retained_chain(
    retained: &WindowsRecursiveRetainedForwarderChainPlanEntry,
) -> Result<WorkingForwarderChain> {
    if retained.visited_targets.is_empty()
        || retained.next_hop_ordinal.checked_add(1) != Some(retained.visited_targets.len())
        || retained.visited_targets.last() != Some(&retained.current_target)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_RETAINED_FORWARDER_SHAPE_CHANGED");
    }
    let mut visited_targets = Vec::with_capacity(retained.visited_targets.len());
    for target in &retained.visited_targets {
        let target = working_target(target)?;
        if visited_targets.contains(&target) {
            bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_CYCLE_DETECTED");
        }
        visited_targets.push(target);
    }
    let current_target = visited_targets
        .last()
        .cloned()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_TARGET_MISSING"))?;
    Ok(WorkingForwarderChain {
        source_import_edge_ordinal: retained.source_import_edge_ordinal,
        next_hop_ordinal: retained.next_hop_ordinal,
        current_target,
        visited_targets,
    })
}

fn apply_current_edge(
    chains: &mut Vec<WorkingForwarderChain>,
    edge: &WindowsRecursiveParsedEdgeRequest,
    target: WorkingForwarderTarget,
) -> Result<()> {
    match (&edge.import_kind, &edge.edge_locator) {
        (
            WindowsRecursiveRequestImportKind::Normal | WindowsRecursiveRequestImportKind::Delay,
            WindowsPostLeaseModuleEdgeLocator::Import {
                source_import_edge_ordinal,
                ..
            },
        ) => record_direct_root(chains, edge, *source_import_edge_ordinal, target),
        (
            WindowsRecursiveRequestImportKind::Forwarder,
            WindowsPostLeaseModuleEdgeLocator::Forwarder {
                source_import_edge_ordinal,
                forwarder_hop_ordinal,
                source_export_name,
                source_export_ordinal,
                ..
            },
        ) => advance_forwarder_chain(
            chains,
            edge,
            *source_import_edge_ordinal,
            *forwarder_hop_ordinal,
            exact_symbol(source_export_name.as_deref(), *source_export_ordinal)?,
            target,
        ),
        _ => bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_EDGE_KIND_CHANGED"),
    }
}

fn apply_final_edge(
    chains: &mut Vec<WorkingForwarderChain>,
    edge: &FinalForwarderEdge<'_>,
    target: WorkingForwarderTarget,
) -> Result<()> {
    match (edge.edge_kind, edge.locator) {
        (
            WindowsLoaderImportEdgeKind::NormalImport | WindowsLoaderImportEdgeKind::DelayImport,
            WindowsLoaderModuleEdgeLocator::BasePrelease {
                locator:
                    WindowsPreliminaryModuleEdgeLocator::Import {
                        source_import_edge_ordinal,
                        edge_evidence_digest,
                        ..
                    },
                ..
            },
        )
        | (
            WindowsLoaderImportEdgeKind::NormalImport | WindowsLoaderImportEdgeKind::DelayImport,
            WindowsLoaderModuleEdgeLocator::SystemPostLease {
                locator:
                    WindowsPostLeaseModuleEdgeLocator::Import {
                        source_import_edge_ordinal,
                        edge_evidence_digest,
                        ..
                    },
                ..
            },
        ) if is_sha256(edge_evidence_digest) => record_final_direct_root(
            chains,
            edge.global_import_edge_ordinal,
            *source_import_edge_ordinal,
            target,
        ),
        (
            WindowsLoaderImportEdgeKind::Forwarder,
            WindowsLoaderModuleEdgeLocator::BasePrelease {
                locator:
                    WindowsPreliminaryModuleEdgeLocator::Forwarder {
                        source_import_edge_ordinal,
                        forwarder_hop_ordinal,
                        source_export_name,
                        source_export_ordinal,
                        hop_evidence_digest,
                    },
                ..
            },
        ) if is_sha256(hop_evidence_digest) => advance_final_forwarder_chain(
            chains,
            edge,
            *source_import_edge_ordinal,
            *forwarder_hop_ordinal,
            exact_symbol(source_export_name.as_deref(), *source_export_ordinal)?,
            target,
        ),
        (
            WindowsLoaderImportEdgeKind::Forwarder,
            WindowsLoaderModuleEdgeLocator::SystemPostLease {
                locator:
                    WindowsPostLeaseModuleEdgeLocator::Forwarder {
                        source_import_edge_ordinal,
                        forwarder_hop_ordinal,
                        source_export_name,
                        source_export_ordinal,
                        hop_evidence_digest,
                    },
                ..
            },
        ) if is_sha256(hop_evidence_digest) => advance_final_forwarder_chain(
            chains,
            edge,
            *source_import_edge_ordinal,
            *forwarder_hop_ordinal,
            exact_symbol(source_export_name.as_deref(), *source_export_ordinal)?,
            target,
        ),
        _ => bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_FINAL_EDGE_CHANGED"),
    }
}

fn final_target(edge: &FinalForwarderEdge<'_>) -> Result<WorkingForwarderTarget> {
    Ok(WorkingForwarderTarget {
        node: edge.target.clone(),
        symbol: exact_symbol(edge.imported_symbol_name, edge.imported_symbol_ordinal)?,
    })
}

fn record_final_direct_root(
    chains: &mut Vec<WorkingForwarderChain>,
    global_import_edge_ordinal: usize,
    source_import_edge_ordinal: usize,
    target: WorkingForwarderTarget,
) -> Result<()> {
    if source_import_edge_ordinal != global_import_edge_ordinal
        || chains
            .iter()
            .any(|chain| chain.source_import_edge_ordinal == source_import_edge_ordinal)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_FINAL_ROOT_CHANGED");
    }
    chains.push(WorkingForwarderChain {
        source_import_edge_ordinal,
        next_hop_ordinal: 0,
        current_target: target.clone(),
        visited_targets: vec![target],
    });
    Ok(())
}

fn advance_final_forwarder_chain(
    chains: &mut [WorkingForwarderChain],
    edge: &FinalForwarderEdge<'_>,
    source_import_edge_ordinal: usize,
    forwarder_hop_ordinal: usize,
    source_symbol: WorkingForwarderSymbol,
    target: WorkingForwarderTarget,
) -> Result<()> {
    let chain = chains
        .iter_mut()
        .find(|chain| chain.source_import_edge_ordinal == source_import_edge_ordinal)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_FINAL_ROOT_MISSING"))?;
    if source_import_edge_ordinal >= edge.global_import_edge_ordinal
        || forwarder_hop_ordinal != chain.next_hop_ordinal
        || edge.importer != &chain.current_target.node
        || source_symbol != chain.current_target.symbol
        || chain.visited_targets.contains(&target)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_FINAL_CHAIN_CHANGED");
    }
    chain.next_hop_ordinal = chain
        .next_hop_ordinal
        .checked_add(1)
        .ok_or_else(count_overflow)?;
    chain.current_target = target.clone();
    chain.visited_targets.push(target);
    Ok(())
}

fn record_direct_root(
    chains: &mut Vec<WorkingForwarderChain>,
    edge: &WindowsRecursiveParsedEdgeRequest,
    source_import_edge_ordinal: usize,
    target: WorkingForwarderTarget,
) -> Result<()> {
    if source_import_edge_ordinal != edge.global_import_edge_ordinal
        || chains
            .iter()
            .any(|chain| chain.source_import_edge_ordinal == source_import_edge_ordinal)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_ROOT_CHANGED");
    }
    chains.push(WorkingForwarderChain {
        source_import_edge_ordinal,
        next_hop_ordinal: 0,
        current_target: target.clone(),
        visited_targets: vec![target],
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn advance_forwarder_chain(
    chains: &mut [WorkingForwarderChain],
    edge: &WindowsRecursiveParsedEdgeRequest,
    source_import_edge_ordinal: usize,
    forwarder_hop_ordinal: usize,
    source_symbol: WorkingForwarderSymbol,
    target: WorkingForwarderTarget,
) -> Result<()> {
    let chain = chains
        .iter_mut()
        .find(|chain| chain.source_import_edge_ordinal == source_import_edge_ordinal)
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_ROOT_MISSING"))?;
    if source_import_edge_ordinal >= edge.global_import_edge_ordinal
        || forwarder_hop_ordinal != chain.next_hop_ordinal
        || edge.importer != chain.current_target.node
        || source_symbol != chain.current_target.symbol
        || chain.visited_targets.contains(&target)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_CHAIN_CHANGED");
    }
    chain.next_hop_ordinal = chain
        .next_hop_ordinal
        .checked_add(1)
        .ok_or_else(count_overflow)?;
    chain.current_target = target.clone();
    chain.visited_targets.push(target);
    Ok(())
}

fn resolved_target(
    resolved: &AuthenticatedWindowsRecursiveWaveResolutionPlan,
    module_request_ordinal: usize,
) -> Result<&WindowsLoaderModuleNode> {
    let mut modules = resolved
        .module_resolutions
        .iter()
        .filter(|module| module.module_request_ordinal == module_request_ordinal);
    let module = modules
        .next()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_MODULE_MISSING"))?;
    if modules.next().is_some() {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_MODULE_REPEATED");
    }
    let mut owners = resolved
        .route_owners
        .iter()
        .filter(|owner| plan_owner_validation::terminal_uses_owner(&module.terminal, &owner.owner));
    let owner = owners
        .next()
        .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_OWNER_MISSING"))?;
    if owners.next().is_some() {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_OWNER_AMBIGUOUS");
    }
    Ok(&owner.target)
}

fn retained_entries(
    mut chains: Vec<WorkingForwarderChain>,
) -> Result<Vec<WindowsRecursiveRetainedForwarderChainPlanEntry>> {
    chains.sort_by_key(|chain| chain.source_import_edge_ordinal);
    chains
        .into_iter()
        .map(|chain| {
            let mut entry = WindowsRecursiveRetainedForwarderChainPlanEntry {
                source_import_edge_ordinal: chain.source_import_edge_ordinal,
                next_hop_ordinal: chain.next_hop_ordinal,
                current_target: contract_target(&chain.current_target),
                visited_targets: chain.visited_targets.iter().map(contract_target).collect(),
                chain_binding_digest: String::new(),
            };
            entry.chain_binding_digest =
                plan_digest::retained_forwarder_chain_binding_digest(&entry)?;
            Ok(entry)
        })
        .collect()
}

fn contract_target(target: &WorkingForwarderTarget) -> WindowsRecursiveForwarderTargetRef {
    WindowsRecursiveForwarderTargetRef {
        node: target.node.clone(),
        symbol: WindowsRecursiveForwarderSymbolRef {
            name: target.symbol.name.clone(),
            ordinal: target.symbol.ordinal,
        },
    }
}

fn final_forwarder_edges(
    resolution: &SealedWindowsLoaderResolutionAuthority,
) -> Vec<FinalForwarderEdge<'_>> {
    resolution
        .package_module_bindings
        .iter()
        .map(|binding| FinalForwarderEdge {
            module_request_ordinal: binding.module_request_ordinal,
            global_import_edge_ordinal: binding.global_import_edge_ordinal,
            locator: &binding.edge_locator,
            importer: &binding.importer,
            edge_kind: binding.edge_kind,
            imported_symbol_name: binding.imported_symbol_name.as_deref(),
            imported_symbol_ordinal: binding.imported_symbol_ordinal,
            target: WindowsLoaderModuleNode::PackageFile {
                package_file_ordinal: binding.resolved_package_file_ordinal,
            },
        })
        .chain(resolution.system_module_bindings.iter().map(|binding| {
            FinalForwarderEdge {
                module_request_ordinal: binding.module_request_ordinal,
                global_import_edge_ordinal: binding.global_import_edge_ordinal,
                locator: &binding.edge_locator,
                importer: &binding.importer,
                edge_kind: binding.edge_kind,
                imported_symbol_name: binding.imported_symbol_name.as_deref(),
                imported_symbol_ordinal: binding.imported_symbol_ordinal,
                target: crate::node_agent_compute_plugin_host::runtime_loader_load_set::pe_graph_validation::system_binding_target_node(binding),
            }
        }))
        .collect()
}

fn working_target(target: &WindowsRecursiveForwarderTargetRef) -> Result<WorkingForwarderTarget> {
    if !module_node_shape_valid(&target.node) {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_TARGET_CHANGED");
    }
    Ok(WorkingForwarderTarget {
        node: target.node.clone(),
        symbol: exact_symbol(target.symbol.name.as_deref(), target.symbol.ordinal)?,
    })
}

fn exact_symbol(name: Option<&str>, ordinal: Option<u16>) -> Result<WorkingForwarderSymbol> {
    if !symbol_is_exact(name, ordinal) {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_SYMBOL_CHANGED");
    }
    Ok(WorkingForwarderSymbol {
        name: name.map(str::to_owned),
        ordinal,
    })
}

fn module_node_shape_valid(node: &WindowsLoaderModuleNode) -> bool {
    match node {
        WindowsLoaderModuleNode::PackageFile { .. } => true,
        WindowsLoaderModuleNode::SystemComponent {
            component_identity_digest,
        }
        | WindowsLoaderModuleNode::ApiSetHost {
            component_identity_digest,
        } => is_sha256(component_identity_digest),
        WindowsLoaderModuleNode::KnownDllSection {
            section_identity_digest,
        } => is_sha256(section_identity_digest),
        WindowsLoaderModuleNode::SideBySideAssembly {
            assembly_identity_digest,
        } => is_sha256(assembly_identity_digest),
    }
}

fn count_overflow() -> anyhow::Error {
    anyhow!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FORWARDER_COUNT_OVERFLOW")
}
